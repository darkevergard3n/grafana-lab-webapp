// =============================================================================
// INVENTORY SERVICE - Main Entry Point
// =============================================================================
// This is the main entry point for the Rust-based Inventory Service.
// 
// WHAT THIS SERVICE DOES:
// - Manages product inventory (stock levels by SKU)
// - Provides APIs to check, reserve, and release stock
// - Exposes Prometheus metrics for observability
// - Caches frequently accessed data in Redis
//
// LEARNING GOALS:
// - Understand Rust async programming with Tokio
// - Learn Axum web framework patterns
// - See how Prometheus metrics work in Rust
// - Understand error handling in Rust
// =============================================================================

// -----------------------------------------------------------------------------
// MODULE DECLARATIONS
// -----------------------------------------------------------------------------
// In Rust, we organize code into modules. Each `mod` statement tells the
// compiler to look for a file or directory with that name.
mod config;      // Configuration loading (config.rs)
mod db;          // Database operations (db.rs)
mod handlers;    // HTTP request handlers (handlers.rs)
mod metrics;     // Prometheus metrics setup (metrics.rs)
mod models;      // Data structures (models.rs)
mod error;       // Error types (error.rs)

// -----------------------------------------------------------------------------
// IMPORTS (use statements)
// -----------------------------------------------------------------------------
// Rust uses `use` to bring items into scope. This is similar to `import` in
// other languages.

use axum::{
    // Router is used to define URL routes
    routing::{get, post},
    Router,
};

// Extension allows sharing state across request handlers
use std::sync::Arc;

// Tower-HTTP provides common HTTP middleware
use tower_http::{
    cors::{Any, CorsLayer},  // CORS handling
    trace::TraceLayer,        // Request tracing/logging
};

// Tracing is Rust's logging framework
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Our custom modules
use crate::config::Config;
use crate::db::Database;
use crate::metrics::setup_metrics;

// -----------------------------------------------------------------------------
// APPLICATION STATE
// -----------------------------------------------------------------------------
// This struct holds shared state that's available to all request handlers.
// Arc (Atomic Reference Counting) allows safe sharing across async tasks.
//
// LEARNING NOTE:
// In Rust, we can't just share mutable data across threads. We need to use
// thread-safe types like Arc<T> (for read-only sharing) or Arc<Mutex<T>>
// (for mutable sharing).
#[derive(Clone)]
pub struct AppState {
    // Database connection pool
    // Pool manages multiple connections for concurrent requests
    pub db: Database,
    
    // Redis connection for caching
    pub redis: redis::aio::ConnectionManager,
    
    // Prometheus metrics handle
    // Used to render metrics in Prometheus format
    pub metrics_handle: metrics_exporter_prometheus::PrometheusHandle,
}

// -----------------------------------------------------------------------------
// MAIN FUNCTION
// -----------------------------------------------------------------------------
// The #[tokio::main] attribute transforms this into an async main function.
// Tokio runtime is started automatically.
//
// LEARNING NOTE:
// Rust doesn't have built-in async support in the standard library.
// We use Tokio, which provides an async runtime (event loop, scheduler, etc.)
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // -------------------------------------------------------------------------
    // STEP 1: Load environment variables
    // -------------------------------------------------------------------------
    // dotenvy loads variables from .env file into environment
    // This is useful for local development
    dotenvy::dotenv().ok();  // .ok() ignores errors (file might not exist)

    // -------------------------------------------------------------------------
    // STEP 2: Initialize logging/tracing
    // -------------------------------------------------------------------------
    // Set up structured logging with JSON output
    // RUST_LOG environment variable controls log levels
    // Example: RUST_LOG=info,inventory_service=debug
    tracing_subscriber::registry()
        // Add filter layer (reads RUST_LOG env var)
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,inventory_service=debug".into()),
        )
        // Add JSON formatting layer
        .with(tracing_subscriber::fmt::layer().json())
        // Initialize as the global default
        .init();

    info!("Starting Inventory Service...");

    // -------------------------------------------------------------------------
    // STEP 3: Load configuration
    // -------------------------------------------------------------------------
    // Config::from_env() reads environment variables and returns a Config struct
    // The ? operator propagates errors (returns early if there's an error)
    let config = Config::from_env()?;
    info!(port = config.port, "Configuration loaded");

    // -------------------------------------------------------------------------
    // STEP 4: Set up Prometheus metrics
    // -------------------------------------------------------------------------
    // This creates a metrics recorder and returns a handle for rendering metrics
    let metrics_handle = setup_metrics()?;
    info!("Prometheus metrics initialized");

    // -------------------------------------------------------------------------
    // STEP 5: Connect to PostgreSQL database
    // -------------------------------------------------------------------------
    // Database::connect() creates a connection pool
    // Connection pools reuse connections for better performance
    let db = Database::connect(&config.database_url).await?;
    info!("Connected to PostgreSQL");

    // Run database migrations (create tables if they don't exist)
    db.run_migrations().await?;
    info!("Database migrations completed");

    // -------------------------------------------------------------------------
    // STEP 6: Connect to Redis
    // -------------------------------------------------------------------------
    // ConnectionManager handles reconnection automatically
    let redis_client = redis::Client::open(config.redis_url.as_str())?;
    let redis_conn = redis::aio::ConnectionManager::new(redis_client).await?;
    info!("Connected to Redis");

    // -------------------------------------------------------------------------
    // STEP 7: Create application state
    // -------------------------------------------------------------------------
    // Arc wraps the state so it can be safely shared across request handlers
    let state = Arc::new(AppState {
        db,
        redis: redis_conn,
        metrics_handle,
    });

    // -------------------------------------------------------------------------
    // STEP 8: Define routes
    // -------------------------------------------------------------------------
    // Router maps URL paths to handler functions
    // 
    // LEARNING NOTE:
    // Axum uses a type-safe routing system. The handler function signatures
    // determine what data is extracted from requests automatically.
    let app = Router::new()
        // ----- Health & Readiness Endpoints -----
        // These are used by Kubernetes/Docker for health checks
        .route("/health", get(handlers::health_check))
        .route("/ready", get(handlers::readiness_check))
        
        // ----- Metrics Endpoint -----
        // Prometheus scrapes this endpoint to collect metrics
        .route("/metrics", get(handlers::metrics_handler))
        
        // ----- Inventory API Endpoints -----
        // RESTful API for inventory management
        .route("/api/v1/inventory", get(handlers::list_inventory))
        .route("/api/v1/inventory/:sku", get(handlers::get_item))
        .route("/api/v1/inventory/reserve", post(handlers::reserve_stock))
        .route("/api/v1/inventory/release", post(handlers::release_stock))
        .route("/api/v1/inventory/adjust", post(handlers::adjust_stock))
        .route("/api/v1/inventory/alerts", get(handlers::low_stock_alerts))
        
        // ----- Middleware Layers -----
        // Layers wrap the entire application and process every request
        
        // CORS layer: Allow cross-origin requests
        // This is necessary for the frontend to call this API
        .layer(
            CorsLayer::new()
                .allow_origin(Any)  // Allow any origin (configure for production!)
                .allow_methods(Any) // Allow any HTTP method
                .allow_headers(Any), // Allow any headers
        )
        
        // Trace layer: Log every request
        .layer(TraceLayer::new_for_http())
        
        // Share application state with all handlers
        // with_state() makes state available via State<Arc<AppState>> extractor
        .with_state(state);

    // -------------------------------------------------------------------------
    // STEP 9: Start the HTTP server
    // -------------------------------------------------------------------------
    // Bind to all network interfaces (0.0.0.0) on the configured port
    let addr = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    
    info!(address = %addr, "Inventory Service is listening");
    
    // Start accepting connections
    // This runs forever until the process is terminated
    axum::serve(listener, app).await?;

    Ok(())
}
