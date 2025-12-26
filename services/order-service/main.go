// =============================================================================
// ORDER SERVICE - Main Entry Point (Go + Gin)
// =============================================================================
// This is the main entry point for the Order Service, built with Go and Gin.
//
// WHAT THIS SERVICE DOES:
// - Manages order lifecycle (create, update, cancel)
// - Orchestrates calls to Inventory, Payment, and User services
// - Publishes order events to RabbitMQ
// - Exposes Prometheus metrics
//
// WHY GO?
// - Fast compilation and execution
// - Excellent concurrency (goroutines)
// - Small binary size, perfect for containers
// - Strong standard library for HTTP
//
// LEARNING GOALS:
// - Understand Go project structure
// - Learn Gin web framework
// - See how Prometheus metrics work in Go
// - Understand inter-service communication
// =============================================================================

package main

import (
	"context"
	"database/sql"
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/gin-gonic/gin"
	"github.com/go-redis/redis/v8"
	_ "github.com/lib/pq" // PostgreSQL driver (blank import for side effects)
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promhttp"
	amqp "github.com/rabbitmq/amqp091-go"
)

// =============================================================================
// STRUCTURED JSON LOGGER
// =============================================================================
// Custom logger that outputs JSON for Loki/Grafana
type JSONLog struct {
	Timestamp string                 `json:"timestamp"`
	Level     string                 `json:"level"`
	Service   string                 `json:"service"`
	Message   string                 `json:"message"`
	Fields    map[string]interface{} `json:"fields,omitempty"`
}

func logJSON(level, message string, fields map[string]interface{}) {
	entry := JSONLog{
		Timestamp: time.Now().UTC().Format(time.RFC3339Nano),
		Level:     level,
		Service:   "order-service",
		Message:   message,
		Fields:    fields,
	}
	jsonBytes, _ := json.Marshal(entry)
	fmt.Println(string(jsonBytes))
}

func logInfo(message string, fields map[string]interface{}) {
	logJSON("INFO", message, fields)
}

func logWarn(message string, fields map[string]interface{}) {
	logJSON("WARN", message, fields)
}

func logError(message string, fields map[string]interface{}) {
	logJSON("ERROR", message, fields)
}

// =============================================================================
// GLOBAL VARIABLES
// =============================================================================
// In Go, we often use package-level variables for shared resources.
// These are initialized in main() and used throughout the application.

var (
	// Database connection pool
	db *sql.DB

	// Redis client
	redisClient *redis.Client

	// RabbitMQ connection and channel
	rabbitConn    *amqp.Connection
	rabbitChannel *amqp.Channel

	// Service URLs for inter-service communication
	inventoryServiceURL    string
	paymentServiceURL      string
	userServiceURL         string
	notificationServiceURL string
)

// =============================================================================
// PROMETHEUS METRICS
// =============================================================================
// Define metrics that will be exposed at /metrics endpoint.
//
// METRIC TYPES IN GO:
// - Counter: prometheus.NewCounterVec (only increases)
// - Gauge: prometheus.NewGaugeVec (can increase/decrease)
// - Histogram: prometheus.NewHistogramVec (distribution of values)

var (
	// Counter: Total HTTP requests received
	// Labels: method (GET/POST), endpoint (/api/v1/orders), status (200/500)
	httpRequestsTotal = prometheus.NewCounterVec(
		prometheus.CounterOpts{
			Name: "http_requests_total",
			Help: "Total number of HTTP requests",
		},
		[]string{"method", "endpoint", "status"},
	)

	// Histogram: HTTP request duration
	// Labels: method, endpoint
	httpRequestDuration = prometheus.NewHistogramVec(
		prometheus.HistogramOpts{
			Name: "http_request_duration_seconds",
			Help: "HTTP request latency in seconds",
			// Buckets define the histogram boundaries
			Buckets: []float64{0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10},
		},
		[]string{"method", "endpoint"},
	)

	// Counter: Orders created
	ordersCreatedTotal = prometheus.NewCounter(
		prometheus.CounterOpts{
			Name: "orders_created_total",
			Help: "Total number of orders created",
		},
	)

	// Gauge: Orders by status
	ordersByStatus = prometheus.NewGaugeVec(
		prometheus.GaugeOpts{
			Name: "orders_by_status",
			Help: "Current number of orders by status",
		},
		[]string{"status"},
	)

	// Histogram: Order processing duration
	orderProcessingDuration = prometheus.NewHistogram(
		prometheus.HistogramOpts{
			Name:    "order_processing_duration_seconds",
			Help:    "Time taken to process an order",
			Buckets: []float64{0.1, 0.5, 1, 2, 5, 10, 30},
		},
	)
)

// init() is called automatically before main()
// We use it to register Prometheus metrics
func init() {
	// Register all metrics with Prometheus
	prometheus.MustRegister(httpRequestsTotal)
	prometheus.MustRegister(httpRequestDuration)
	prometheus.MustRegister(ordersCreatedTotal)
	prometheus.MustRegister(ordersByStatus)
	prometheus.MustRegister(orderProcessingDuration)
}

// =============================================================================
// CONFIGURATION
// =============================================================================
// Config holds all configuration values loaded from environment variables.
type Config struct {
	Port           string
	DatabaseURL    string
	RedisURL       string
	RabbitMQURL    string
	InventoryURL   string
	PaymentURL     string
	UserURL        string
	NotificationURL string
}

// LoadConfig reads configuration from environment variables
func LoadConfig() *Config {
	return &Config{
		Port:           getEnv("PORT", "8001"),
		DatabaseURL:    getEnv("DATABASE_URL", "postgres://webapp:webapp@localhost/orderdb?sslmode=disable"),
		RedisURL:       getEnv("REDIS_URL", "redis://localhost:6379/0"),
		RabbitMQURL:    getEnv("RABBITMQ_URL", "amqp://guest:guest@localhost:5672/"),
		InventoryURL:   getEnv("INVENTORY_SERVICE_URL", "http://localhost:8002"),
		PaymentURL:     getEnv("PAYMENT_SERVICE_URL", "http://localhost:8003"),
		UserURL:        getEnv("USER_SERVICE_URL", "http://localhost:8004"),
		NotificationURL: getEnv("NOTIFICATION_SERVICE_URL", "http://localhost:8005"),
	}
}

// getEnv gets an environment variable or returns a default value
func getEnv(key, defaultValue string) string {
	if value := os.Getenv(key); value != "" {
		return value
	}
	return defaultValue
}

// =============================================================================
// MAIN FUNCTION
// =============================================================================
func main() {
	// Load configuration
	config := LoadConfig()
	log.Printf("Starting Order Service on port %s", config.Port)

	// Store service URLs in package variables
	inventoryServiceURL = config.InventoryURL
	paymentServiceURL = config.PaymentURL
	userServiceURL = config.UserURL
	notificationServiceURL = config.NotificationURL

	// -------------------------------------------------------------------------
	// CONNECT TO POSTGRESQL
	// -------------------------------------------------------------------------
	var err error
	db, err = sql.Open("postgres", config.DatabaseURL)
	if err != nil {
		log.Fatalf("Failed to connect to PostgreSQL: %v", err)
	}
	defer db.Close()

	// Configure connection pool
	db.SetMaxOpenConns(25)
	db.SetMaxIdleConns(5)
	db.SetConnMaxLifetime(5 * time.Minute)

	// Test connection
	if err := db.Ping(); err != nil {
		log.Fatalf("Failed to ping PostgreSQL: %v", err)
	}
	log.Println("Connected to PostgreSQL")

	// Run database migrations
	if err := runMigrations(); err != nil {
		log.Fatalf("Failed to run migrations: %v", err)
	}

	// -------------------------------------------------------------------------
	// CONNECT TO REDIS
	// -------------------------------------------------------------------------
	redisOpts, err := redis.ParseURL(config.RedisURL)
	if err != nil {
		log.Fatalf("Failed to parse Redis URL: %v", err)
	}
	redisClient = redis.NewClient(redisOpts)

	// Test Redis connection
	ctx := context.Background()
	if _, err := redisClient.Ping(ctx).Result(); err != nil {
		log.Fatalf("Failed to connect to Redis: %v", err)
	}
	log.Println("Connected to Redis")

	// -------------------------------------------------------------------------
	// CONNECT TO RABBITMQ
	// -------------------------------------------------------------------------
	rabbitConn, err = amqp.Dial(config.RabbitMQURL)
	if err != nil {
		log.Fatalf("Failed to connect to RabbitMQ: %v", err)
	}
	defer rabbitConn.Close()

	rabbitChannel, err = rabbitConn.Channel()
	if err != nil {
		log.Fatalf("Failed to open RabbitMQ channel: %v", err)
	}
	defer rabbitChannel.Close()

	// Declare exchange for order events
	err = rabbitChannel.ExchangeDeclare(
		"orders",  // Exchange name
		"topic",   // Exchange type
		true,      // Durable
		false,     // Auto-deleted
		false,     // Internal
		false,     // No-wait
		nil,       // Arguments
	)
	if err != nil {
		log.Fatalf("Failed to declare RabbitMQ exchange: %v", err)
	}
	log.Println("Connected to RabbitMQ")

	// -------------------------------------------------------------------------
	// SETUP GIN ROUTER
	// -------------------------------------------------------------------------
	// Set Gin to release mode for production (less verbose logging)
	gin.SetMode(gin.ReleaseMode)

	router := gin.New()

	// Add middleware
	router.Use(gin.Recovery())          // Recover from panics
	router.Use(loggingMiddleware())     // Custom logging
	router.Use(metricsMiddleware())     // Prometheus metrics

	// -------------------------------------------------------------------------
	// DEFINE ROUTES
	// -------------------------------------------------------------------------

	// Health check endpoints
	router.GET("/health", healthCheck)
	router.GET("/ready", readinessCheck)

	// Prometheus metrics endpoint
	router.GET("/metrics", gin.WrapH(promhttp.Handler()))

	// Order API endpoints
	api := router.Group("/api/v1")
	{
		orders := api.Group("/orders")
		{
			orders.GET("", listOrders)           // GET /api/v1/orders
			orders.GET("/:id", getOrder)         // GET /api/v1/orders/:id
			orders.POST("", createOrder)         // POST /api/v1/orders
			orders.PUT("/:id", updateOrder)      // PUT /api/v1/orders/:id
			orders.DELETE("/:id", cancelOrder)   // DELETE /api/v1/orders/:id
			orders.POST("/:id/status", updateOrderStatus) // POST /api/v1/orders/:id/status
		}
	}

	// -------------------------------------------------------------------------
	// START SERVER WITH GRACEFUL SHUTDOWN
	// -------------------------------------------------------------------------
	srv := &http.Server{
		Addr:    ":" + config.Port,
		Handler: router,
	}

	// Start server in a goroutine
	go func() {
		log.Printf("Order Service listening on :%s", config.Port)
		if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			log.Fatalf("Server failed: %v", err)
		}
	}()

	// Wait for interrupt signal (Ctrl+C or SIGTERM)
	quit := make(chan os.Signal, 1)
	signal.Notify(quit, syscall.SIGINT, syscall.SIGTERM)
	<-quit
	log.Println("Shutting down server...")

	// Give outstanding requests 30 seconds to complete
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	if err := srv.Shutdown(ctx); err != nil {
		log.Fatalf("Server forced to shutdown: %v", err)
	}

	log.Println("Server exited gracefully")
}

// =============================================================================
// DATABASE MIGRATIONS
// =============================================================================
func runMigrations() error {
	// Create orders table
	_, err := db.Exec(`
		CREATE TABLE IF NOT EXISTS orders (
			id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
			customer_id UUID NOT NULL,
			customer_name VARCHAR(255) NOT NULL,
			customer_email VARCHAR(255) NOT NULL,
			status VARCHAR(50) NOT NULL DEFAULT 'pending',
			total_amount DECIMAL(12, 2) NOT NULL DEFAULT 0,
			currency VARCHAR(3) NOT NULL DEFAULT 'USD',
			shipping_address TEXT,
			notes TEXT,
			created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
			updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
		)
	`)
	if err != nil {
		return fmt.Errorf("failed to create orders table: %w", err)
	}

	// Create order_items table
	_, err = db.Exec(`
		CREATE TABLE IF NOT EXISTS order_items (
			id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
			order_id UUID NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
			sku VARCHAR(50) NOT NULL,
			name VARCHAR(255) NOT NULL,
			quantity INTEGER NOT NULL,
			unit_price DECIMAL(12, 2) NOT NULL,
			total_price DECIMAL(12, 2) NOT NULL,
			created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
		)
	`)
	if err != nil {
		return fmt.Errorf("failed to create order_items table: %w", err)
	}

	// Create indexes
	_, err = db.Exec(`CREATE INDEX IF NOT EXISTS idx_orders_customer_id ON orders(customer_id)`)
	if err != nil {
		return fmt.Errorf("failed to create customer_id index: %w", err)
	}

	_, err = db.Exec(`CREATE INDEX IF NOT EXISTS idx_orders_status ON orders(status)`)
	if err != nil {
		return fmt.Errorf("failed to create status index: %w", err)
	}

	log.Println("Database migrations completed")
	return nil
}

// =============================================================================
// MIDDLEWARE
// =============================================================================

// loggingMiddleware logs each request
func loggingMiddleware() gin.HandlerFunc {
	return func(c *gin.Context) {
		// Start timer
		start := time.Now()
		path := c.Request.URL.Path

		// Process request
		c.Next()

		// Calculate latency
		latency := time.Since(start)

		// Log request details
		log.Printf(
			"[%s] %s %s %d %s",
			c.Request.Method,
			path,
			c.ClientIP(),
			c.Writer.Status(),
			latency,
		)
	}
}

// metricsMiddleware records Prometheus metrics for each request
func metricsMiddleware() gin.HandlerFunc {
	return func(c *gin.Context) {
		start := time.Now()
		path := c.FullPath() // Use route pattern, not actual path
		if path == "" {
			path = c.Request.URL.Path
		}

		// Process request
		c.Next()

		// Record metrics
		duration := time.Since(start).Seconds()
		status := fmt.Sprintf("%d", c.Writer.Status())

		httpRequestsTotal.WithLabelValues(c.Request.Method, path, status).Inc()
		httpRequestDuration.WithLabelValues(c.Request.Method, path).Observe(duration)
	}
}

// =============================================================================
// HEALTH CHECK HANDLERS
// =============================================================================

// healthCheck returns service health status
func healthCheck(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{
		"status":  "ok",
		"service": "order-service",
		"version": "1.0.0",
	})
}

// readinessCheck checks if all dependencies are ready
func readinessCheck(c *gin.Context) {
	// Check database
	dbHealthy := db.Ping() == nil

	// Check Redis
	ctx := context.Background()
	redisHealthy := redisClient.Ping(ctx).Err() == nil

	// Check RabbitMQ
	rabbitHealthy := rabbitConn != nil && !rabbitConn.IsClosed()

	allHealthy := dbHealthy && redisHealthy && rabbitHealthy

	response := gin.H{
		"status": "ready",
		"checks": gin.H{
			"database": dbHealthy,
			"redis":    redisHealthy,
			"rabbitmq": rabbitHealthy,
		},
	}

	if allHealthy {
		c.JSON(http.StatusOK, response)
	} else {
		response["status"] = "not_ready"
		c.JSON(http.StatusServiceUnavailable, response)
	}
}

// =============================================================================
// ORDER HANDLERS (Simplified for brevity - full implementation would be larger)
// =============================================================================

// Order represents an order in the system
type Order struct {
	ID              string      `json:"id"`
	CustomerID      string      `json:"customer_id"`
	CustomerName    string      `json:"customer_name"`
	CustomerEmail   string      `json:"customer_email"`
	Status          string      `json:"status"`
	TotalAmount     float64     `json:"total_amount"`
	Currency        string      `json:"currency"`
	ShippingAddress string      `json:"shipping_address,omitempty"`
	Notes           string      `json:"notes,omitempty"`
	Items           []OrderItem `json:"items,omitempty"`
	CreatedAt       time.Time   `json:"created_at"`
	UpdatedAt       time.Time   `json:"updated_at"`
}

// OrderItem represents an item in an order
type OrderItem struct {
	ID         string  `json:"id"`
	OrderID    string  `json:"order_id"`
	SKU        string  `json:"sku"`
	Name       string  `json:"name"`
	Quantity   int     `json:"quantity"`
	UnitPrice  float64 `json:"unit_price"`
	TotalPrice float64 `json:"total_price"`
}

// CreateOrderRequest is the request body for creating an order
type CreateOrderRequest struct {
	CustomerID      string             `json:"customer_id" binding:"required"`
	CustomerName    string             `json:"customer_name" binding:"required"`
	CustomerEmail   string             `json:"customer_email" binding:"required,email"`
	ShippingAddress string             `json:"shipping_address"`
	Notes           string             `json:"notes"`
	Items           []OrderItemRequest `json:"items" binding:"required,min=1"`
}

// OrderItemRequest is an item in a create order request
type OrderItemRequest struct {
	SKU       string  `json:"sku" binding:"required"`
	Name      string  `json:"name" binding:"required"`
	Quantity  int     `json:"quantity" binding:"required,min=1"`
	UnitPrice float64 `json:"unit_price" binding:"required,min=0"`
}

// listOrders returns a paginated list of orders
func listOrders(c *gin.Context) {
	// Parse pagination parameters
	page := 1
	perPage := 20
	if p := c.Query("page"); p != "" {
		fmt.Sscanf(p, "%d", &page)
	}
	if pp := c.Query("per_page"); pp != "" {
		fmt.Sscanf(pp, "%d", &perPage)
	}

	logInfo("Listing orders", map[string]interface{}{
		"page":     page,
		"per_page": perPage,
	})

	offset := (page - 1) * perPage

	// Query orders
	rows, err := db.Query(`
		SELECT id, customer_id, customer_name, customer_email, status,
		       total_amount, currency, shipping_address, notes, created_at, updated_at
		FROM orders
		ORDER BY created_at DESC
		LIMIT $1 OFFSET $2
	`, perPage, offset)
	if err != nil {
		logError("Failed to list orders", map[string]interface{}{
			"error": err.Error(),
		})
		c.JSON(http.StatusInternalServerError, gin.H{"error": "Database error"})
		return
	}
	defer rows.Close()

	var orders []Order
	for rows.Next() {
		var o Order
		var shippingAddr, notes sql.NullString
		err := rows.Scan(
			&o.ID, &o.CustomerID, &o.CustomerName, &o.CustomerEmail,
			&o.Status, &o.TotalAmount, &o.Currency,
			&shippingAddr, &notes, &o.CreatedAt, &o.UpdatedAt,
		)
		if err != nil {
			continue
		}
		o.ShippingAddress = shippingAddr.String
		o.Notes = notes.String
		orders = append(orders, o)
	}

	// Get total count
	var total int
	db.QueryRow("SELECT COUNT(*) FROM orders").Scan(&total)

	logInfo("Orders listed successfully", map[string]interface{}{
		"page":        page,
		"per_page":    perPage,
		"returned":    len(orders),
		"total":       total,
	})

	c.JSON(http.StatusOK, gin.H{
		"orders":   orders,
		"total":    total,
		"page":     page,
		"per_page": perPage,
	})
}

// getOrder returns a single order by ID
func getOrder(c *gin.Context) {
	id := c.Param("id")

	logInfo("Fetching order", map[string]interface{}{
		"order_id": id,
	})

	var o Order
	var shippingAddr, notes sql.NullString
	err := db.QueryRow(`
		SELECT id, customer_id, customer_name, customer_email, status,
		       total_amount, currency, shipping_address, notes, created_at, updated_at
		FROM orders WHERE id = $1
	`, id).Scan(
		&o.ID, &o.CustomerID, &o.CustomerName, &o.CustomerEmail,
		&o.Status, &o.TotalAmount, &o.Currency,
		&shippingAddr, &notes, &o.CreatedAt, &o.UpdatedAt,
	)
	if err == sql.ErrNoRows {
		logWarn("Order not found", map[string]interface{}{
			"order_id": id,
		})
		c.JSON(http.StatusNotFound, gin.H{"error": "Order not found"})
		return
	}
	if err != nil {
		logError("Database error fetching order", map[string]interface{}{
			"order_id": id,
			"error":    err.Error(),
		})
		c.JSON(http.StatusInternalServerError, gin.H{"error": "Database error"})
		return
	}

	o.ShippingAddress = shippingAddr.String
	o.Notes = notes.String

	// Get order items
	rows, err := db.Query(`
		SELECT id, order_id, sku, name, quantity, unit_price, total_price
		FROM order_items WHERE order_id = $1
	`, id)
	if err == nil {
		defer rows.Close()
		for rows.Next() {
			var item OrderItem
			rows.Scan(&item.ID, &item.OrderID, &item.SKU, &item.Name,
				&item.Quantity, &item.UnitPrice, &item.TotalPrice)
			o.Items = append(o.Items, item)
		}
	}

	logInfo("Order fetched successfully", map[string]interface{}{
		"order_id":    id,
		"status":      o.Status,
		"items_count": len(o.Items),
	})

	c.JSON(http.StatusOK, o)
}

// createOrder creates a new order
func createOrder(c *gin.Context) {
	start := time.Now()

	var req CreateOrderRequest
	if err := c.ShouldBindJSON(&req); err != nil {
		logWarn("Invalid order request", map[string]interface{}{
			"error": err.Error(),
		})
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	// Log incoming order request
	logInfo("Creating new order", map[string]interface{}{
		"customer_id":    req.CustomerID,
		"customer_name":  req.CustomerName,
		"customer_email": req.CustomerEmail,
		"items_count":    len(req.Items),
	})

	// Calculate total
	var totalAmount float64
	for _, item := range req.Items {
		totalAmount += float64(item.Quantity) * item.UnitPrice
	}

	// Insert order
	var orderID string
	err := db.QueryRow(`
		INSERT INTO orders (customer_id, customer_name, customer_email, 
		                    shipping_address, notes, total_amount, status)
		VALUES ($1, $2, $3, $4, $5, $6, 'pending')
		RETURNING id
	`, req.CustomerID, req.CustomerName, req.CustomerEmail,
		req.ShippingAddress, req.Notes, totalAmount).Scan(&orderID)
	if err != nil {
		logError("Failed to create order in database", map[string]interface{}{
			"error":       err.Error(),
			"customer_id": req.CustomerID,
		})
		c.JSON(http.StatusInternalServerError, gin.H{"error": "Failed to create order"})
		return
	}

	// Insert order items
	for _, item := range req.Items {
		itemTotal := float64(item.Quantity) * item.UnitPrice
		_, err := db.Exec(`
			INSERT INTO order_items (order_id, sku, name, quantity, unit_price, total_price)
			VALUES ($1, $2, $3, $4, $5, $6)
		`, orderID, item.SKU, item.Name, item.Quantity, item.UnitPrice, itemTotal)
		if err != nil {
			logWarn("Failed to insert order item", map[string]interface{}{
				"order_id": orderID,
				"sku":      item.SKU,
				"error":    err.Error(),
			})
		}
	}

	// Update metrics
	ordersCreatedTotal.Inc()
	orderProcessingDuration.Observe(time.Since(start).Seconds())

	// Publish order created event
	publishOrderEvent("order.created", orderID)

	// Log successful creation
	logInfo("Order created successfully", map[string]interface{}{
		"order_id":      orderID,
		"customer_id":   req.CustomerID,
		"total_amount":  totalAmount,
		"items_count":   len(req.Items),
		"duration_ms":   time.Since(start).Milliseconds(),
	})

	c.JSON(http.StatusCreated, gin.H{
		"id":      orderID,
		"status":  "pending",
		"total":   totalAmount,
		"message": "Order created successfully",
	})
}

// updateOrder updates an existing order
func updateOrder(c *gin.Context) {
	id := c.Param("id")

	var req struct {
		ShippingAddress string `json:"shipping_address"`
		Notes           string `json:"notes"`
	}
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	result, err := db.Exec(`
		UPDATE orders 
		SET shipping_address = $1, notes = $2, updated_at = NOW()
		WHERE id = $3
	`, req.ShippingAddress, req.Notes, id)
	if err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "Database error"})
		return
	}

	rowsAffected, _ := result.RowsAffected()
	if rowsAffected == 0 {
		c.JSON(http.StatusNotFound, gin.H{"error": "Order not found"})
		return
	}

	publishOrderEvent("order.updated", id)

	c.JSON(http.StatusOK, gin.H{"message": "Order updated successfully"})
}

// updateOrderStatus updates the status of an order
func updateOrderStatus(c *gin.Context) {
	id := c.Param("id")

	var req struct {
		Status string `json:"status" binding:"required"`
	}
	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	// Validate status
	validStatuses := map[string]bool{
		"pending": true, "processing": true, "shipped": true,
		"delivered": true, "cancelled": true,
	}
	if !validStatuses[req.Status] {
		logWarn("Invalid order status attempted", map[string]interface{}{
			"order_id":         id,
			"attempted_status": req.Status,
		})
		c.JSON(http.StatusBadRequest, gin.H{"error": "Invalid status"})
		return
	}

	logInfo("Updating order status", map[string]interface{}{
		"order_id":   id,
		"new_status": req.Status,
	})

	result, err := db.Exec(`
		UPDATE orders SET status = $1, updated_at = NOW() WHERE id = $2
	`, req.Status, id)
	if err != nil {
		logError("Failed to update order status", map[string]interface{}{
			"order_id": id,
			"status":   req.Status,
			"error":    err.Error(),
		})
		c.JSON(http.StatusInternalServerError, gin.H{"error": "Database error"})
		return
	}

	rowsAffected, _ := result.RowsAffected()
	if rowsAffected == 0 {
		logWarn("Order not found for status update", map[string]interface{}{
			"order_id": id,
		})
		c.JSON(http.StatusNotFound, gin.H{"error": "Order not found"})
		return
	}

	publishOrderEvent("order.status."+req.Status, id)

	logInfo("Order status updated successfully", map[string]interface{}{
		"order_id":   id,
		"new_status": req.Status,
	})

	c.JSON(http.StatusOK, gin.H{
		"message": "Order status updated",
		"status":  req.Status,
	})
}

// cancelOrder cancels an order
func cancelOrder(c *gin.Context) {
	id := c.Param("id")

	logInfo("Attempting to cancel order", map[string]interface{}{
		"order_id": id,
	})

	result, err := db.Exec(`
		UPDATE orders SET status = 'cancelled', updated_at = NOW()
		WHERE id = $1 AND status NOT IN ('shipped', 'delivered')
	`, id)
	if err != nil {
		logError("Failed to cancel order", map[string]interface{}{
			"order_id": id,
			"error":    err.Error(),
		})
		c.JSON(http.StatusInternalServerError, gin.H{"error": "Database error"})
		return
	}

	rowsAffected, _ := result.RowsAffected()
	if rowsAffected == 0 {
		logWarn("Order cannot be cancelled", map[string]interface{}{
			"order_id": id,
			"reason":   "Order not found or already shipped/delivered",
		})
		c.JSON(http.StatusBadRequest, gin.H{
			"error": "Order not found or cannot be cancelled",
		})
		return
	}

	publishOrderEvent("order.cancelled", id)

	logInfo("Order cancelled successfully", map[string]interface{}{
		"order_id": id,
	})

	c.JSON(http.StatusOK, gin.H{"message": "Order cancelled successfully"})
}

// =============================================================================
// RABBITMQ HELPERS
// =============================================================================

// publishOrderEvent publishes an event to the orders exchange
func publishOrderEvent(eventType, orderID string) {
	if rabbitChannel == nil {
		return
	}

	body := fmt.Sprintf(`{"event":"%s","order_id":"%s","timestamp":"%s"}`,
		eventType, orderID, time.Now().Format(time.RFC3339))

	err := rabbitChannel.PublishWithContext(
		context.Background(),
		"orders",   // Exchange
		eventType,  // Routing key
		false,      // Mandatory
		false,      // Immediate
		amqp.Publishing{
			ContentType: "application/json",
			Body:        []byte(body),
		},
	)
	if err != nil {
		log.Printf("Failed to publish order event: %v", err)
	}
}
