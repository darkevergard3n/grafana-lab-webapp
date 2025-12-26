/**
 * =============================================================================
 * NOTIFICATION SERVICE - Main Entry Point (Node.js + Express)
 * =============================================================================
 * This is the main entry point for the Notification Service.
 *
 * WHAT THIS SERVICE DOES:
 * - Consume order events from RabbitMQ
 * - Send email notifications (mock)
 * - WebSocket for real-time UI updates
 * - Expose Prometheus metrics
 *
 * WHY NODE.JS?
 * - Event-driven architecture perfect for message consumption
 * - Non-blocking I/O for high throughput
 * - Excellent WebSocket support
 * - Rich NPM ecosystem for notifications
 *
 * LEARNING GOALS:
 * - Understand Node.js event loop
 * - Learn Express patterns
 * - See WebSocket implementation
 * - Understand message queue consumption
 * =============================================================================
 */

// =============================================================================
// IMPORTS
// =============================================================================

const express = require('express');
const http = require('http');
const WebSocket = require('ws');
const amqp = require('amqplib');
const Redis = require('ioredis');
const promClient = require('prom-client');

// =============================================================================
// CONFIGURATION
// =============================================================================
/**
 * Configuration object loaded from environment variables.
 * Default values are provided for local development.
 */
const config = {
  port: process.env.PORT || 8005,
  nodeEnv: process.env.NODE_ENV || 'development',
  logLevel: process.env.LOG_LEVEL || 'info',
  
  // RabbitMQ connection
  rabbitmqUrl: process.env.RABBITMQ_URL || 'amqp://guest:guest@localhost:5672',
  
  // Redis connection
  redisUrl: process.env.REDIS_URL || 'redis://localhost:6379/3',
};

// =============================================================================
// LOGGING
// =============================================================================
/**
 * Simple JSON logger for structured logging.
 * In production, you might use winston or pino.
 */
const logger = {
  /**
   * Log an info message
   * @param {string} message - Log message
   * @param {object} extra - Additional fields to log
   */
  info: (message, extra = {}) => {
    console.log(JSON.stringify({
      timestamp: new Date().toISOString(),
      level: 'INFO',
      service: 'notification-service',
      message,
      ...extra,
    }));
  },
  
  /**
   * Log a warning message
   */
  warn: (message, extra = {}) => {
    console.log(JSON.stringify({
      timestamp: new Date().toISOString(),
      level: 'WARN',
      service: 'notification-service',
      message,
      ...extra,
    }));
  },
  
  /**
   * Log an error message
   */
  error: (message, extra = {}) => {
    console.error(JSON.stringify({
      timestamp: new Date().toISOString(),
      level: 'ERROR',
      service: 'notification-service',
      message,
      ...extra,
    }));
  },
};

// =============================================================================
// PROMETHEUS METRICS
// =============================================================================
/**
 * Prometheus metrics setup.
 * We use the official prom-client library.
 */

// Create a Registry to register metrics
const register = new promClient.Registry();

// Add default metrics (CPU, memory, etc.)
promClient.collectDefaultMetrics({ register });

// HTTP request counter
const httpRequestsTotal = new promClient.Counter({
  name: 'http_requests_total',
  help: 'Total HTTP requests',
  labelNames: ['method', 'endpoint', 'status'],
  registers: [register],
});

// HTTP request duration histogram
const httpRequestDuration = new promClient.Histogram({
  name: 'http_request_duration_seconds',
  help: 'HTTP request latency in seconds',
  labelNames: ['method', 'endpoint'],
  buckets: [0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10],
  registers: [register],
});

// Notifications sent counter
const notificationsSentTotal = new promClient.Counter({
  name: 'notifications_sent_total',
  help: 'Total notifications sent',
  labelNames: ['type', 'status'], // type: email/sms, status: success/failed
  registers: [register],
});

// Notification queue size gauge
const notificationQueueSize = new promClient.Gauge({
  name: 'notification_queue_size',
  help: 'Current size of notification queue',
  registers: [register],
});

// WebSocket connections gauge
const websocketConnections = new promClient.Gauge({
  name: 'websocket_connections_current',
  help: 'Current number of WebSocket connections',
  registers: [register],
});

// Email send duration histogram
const emailSendDuration = new promClient.Histogram({
  name: 'email_send_duration_seconds',
  help: 'Email send latency in seconds',
  buckets: [0.1, 0.5, 1, 2, 5, 10],
  registers: [register],
});

// =============================================================================
// GLOBAL CONNECTIONS
// =============================================================================

let rabbitConnection = null;
let rabbitChannel = null;
let redisClient = null;

// Store WebSocket connections
const wsClients = new Set();

// =============================================================================
// EXPRESS APP SETUP
// =============================================================================

const app = express();

// Parse JSON bodies
app.use(express.json());

// Metrics middleware
app.use((req, res, next) => {
  const start = Date.now();
  
  // After response is sent
  res.on('finish', () => {
    const duration = (Date.now() - start) / 1000;
    const path = req.route ? req.route.path : req.path;
    
    httpRequestsTotal.labels(req.method, path, res.statusCode.toString()).inc();
    httpRequestDuration.labels(req.method, path).observe(duration);
  });
  
  next();
});

// =============================================================================
// HEALTH CHECK ENDPOINTS
// =============================================================================

/**
 * Liveness probe - Is the service running?
 * GET /health
 */
app.get('/health', (req, res) => {
  res.json({
    status: 'ok',
    service: 'notification-service',
    version: '1.0.0',
  });
});

/**
 * Readiness probe - Is the service ready?
 * GET /ready
 */
app.get('/ready', async (req, res) => {
  const rabbitHealthy = rabbitConnection !== null && rabbitConnection.connection.serverProperties;
  const redisHealthy = redisClient !== null && redisClient.status === 'ready';
  
  const allHealthy = rabbitHealthy && redisHealthy;
  
  const response = {
    status: allHealthy ? 'ready' : 'not_ready',
    checks: {
      rabbitmq: rabbitHealthy,
      redis: redisHealthy,
    },
  };
  
  if (allHealthy) {
    res.json(response);
  } else {
    res.status(503).json(response);
  }
});

// =============================================================================
// METRICS ENDPOINT
// =============================================================================

/**
 * Prometheus metrics endpoint
 * GET /metrics
 */
app.get('/metrics', async (req, res) => {
  try {
    res.set('Content-Type', register.contentType);
    res.end(await register.metrics());
  } catch (err) {
    res.status(500).end(err.message);
  }
});

// =============================================================================
// NOTIFICATION API ENDPOINTS
// =============================================================================

/**
 * Send a notification manually
 * POST /api/v1/notifications/send
 */
app.post('/api/v1/notifications/send', async (req, res) => {
  const { type, recipient, subject, body, order_id } = req.body;
  
  if (!type || !recipient || !body) {
    return res.status(400).json({
      error: 'Missing required fields: type, recipient, body',
    });
  }
  
  try {
    // Simulate sending notification
    const notification = await sendNotification({
      type,
      recipient,
      subject,
      body,
      order_id,
    });
    
    res.status(201).json(notification);
  } catch (err) {
    logger.error('Failed to send notification', { error: err.message });
    res.status(500).json({ error: 'Failed to send notification' });
  }
});

/**
 * List recent notifications
 * GET /api/v1/notifications
 */
app.get('/api/v1/notifications', async (req, res) => {
  try {
    // Get recent notifications from Redis
    const notifications = await redisClient.lrange('notifications:recent', 0, 49);
    res.json(notifications.map(n => JSON.parse(n)));
  } catch (err) {
    logger.error('Failed to list notifications', { error: err.message });
    res.status(500).json({ error: 'Failed to list notifications' });
  }
});

// =============================================================================
// NOTIFICATION SENDING
// =============================================================================

/**
 * Send a notification (mock implementation)
 * @param {object} params - Notification parameters
 */
async function sendNotification({ type, recipient, subject, body, order_id }) {
  const start = Date.now();
  
  logger.info('Sending notification', { type, recipient, order_id });
  
  // Simulate email sending delay (100-500ms)
  await new Promise(resolve => setTimeout(resolve, 100 + Math.random() * 400));
  
  const notification = {
    id: `notif-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
    type,
    recipient,
    subject,
    body,
    order_id,
    status: 'sent',
    sent_at: new Date().toISOString(),
  };
  
  // Store in Redis
  await redisClient.lpush('notifications:recent', JSON.stringify(notification));
  await redisClient.ltrim('notifications:recent', 0, 999); // Keep last 1000
  
  // Record metrics
  const duration = (Date.now() - start) / 1000;
  emailSendDuration.observe(duration);
  notificationsSentTotal.labels(type, 'success').inc();
  
  // Broadcast to WebSocket clients
  broadcastToClients({
    event: 'notification_sent',
    data: notification,
  });
  
  return notification;
}

// =============================================================================
// RABBITMQ CONSUMER
// =============================================================================

/**
 * Connect to RabbitMQ and start consuming messages
 */
async function connectRabbitMQ() {
  try {
    // Connect to RabbitMQ
    rabbitConnection = await amqp.connect(config.rabbitmqUrl);
    rabbitChannel = await rabbitConnection.createChannel();
    
    logger.info('Connected to RabbitMQ');
    
    // Declare queue for order events
    const queueName = 'notification-service-orders';
    await rabbitChannel.assertQueue(queueName, { durable: true });
    
    // Bind queue to orders exchange
    await rabbitChannel.assertExchange('orders', 'topic', { durable: true });
    await rabbitChannel.bindQueue(queueName, 'orders', 'order.*');
    
    // Start consuming messages
    rabbitChannel.consume(queueName, async (msg) => {
      if (msg) {
        try {
          const event = JSON.parse(msg.content.toString());
          await handleOrderEvent(event);
          rabbitChannel.ack(msg);
        } catch (err) {
          logger.error('Failed to process message', { error: err.message });
          // Reject and don't requeue (dead letter)
          rabbitChannel.nack(msg, false, false);
        }
      }
    });
    
    logger.info('Started consuming order events');
    
    // Handle connection close
    rabbitConnection.on('close', () => {
      logger.warn('RabbitMQ connection closed, reconnecting...');
      setTimeout(connectRabbitMQ, 5000);
    });
    
  } catch (err) {
    logger.error('Failed to connect to RabbitMQ', { error: err.message });
    setTimeout(connectRabbitMQ, 5000);
  }
}

/**
 * Handle an order event from RabbitMQ
 * @param {object} event - Order event
 */
async function handleOrderEvent(event) {
  logger.info('Received order event', { event: event.event, order_id: event.order_id });
  
  // Map event types to notification templates
  const templates = {
    'order.created': {
      subject: 'Order Confirmation',
      body: `Your order ${event.order_id} has been received and is being processed.`,
    },
    'order.status.processing': {
      subject: 'Order Processing',
      body: `Your order ${event.order_id} is now being processed.`,
    },
    'order.status.shipped': {
      subject: 'Order Shipped',
      body: `Great news! Your order ${event.order_id} has been shipped.`,
    },
    'order.status.delivered': {
      subject: 'Order Delivered',
      body: `Your order ${event.order_id} has been delivered. Enjoy!`,
    },
    'order.cancelled': {
      subject: 'Order Cancelled',
      body: `Your order ${event.order_id} has been cancelled.`,
    },
  };
  
  const template = templates[event.event];
  if (template) {
    await sendNotification({
      type: 'email',
      recipient: 'customer@example.com', // In real system, get from order
      subject: template.subject,
      body: template.body,
      order_id: event.order_id,
    });
  }
  
  // Broadcast event to WebSocket clients
  broadcastToClients({
    event: 'order_update',
    data: event,
  });
}

// =============================================================================
// REDIS CONNECTION
// =============================================================================

/**
 * Connect to Redis
 */
async function connectRedis() {
  redisClient = new Redis(config.redisUrl);
  
  redisClient.on('connect', () => {
    logger.info('Connected to Redis');
  });
  
  redisClient.on('error', (err) => {
    logger.error('Redis error', { error: err.message });
  });
}

// =============================================================================
// WEBSOCKET SERVER
// =============================================================================

/**
 * Broadcast a message to all connected WebSocket clients
 * @param {object} message - Message to broadcast
 */
function broadcastToClients(message) {
  const data = JSON.stringify(message);
  
  wsClients.forEach((client) => {
    if (client.readyState === WebSocket.OPEN) {
      client.send(data);
    }
  });
}

/**
 * Setup WebSocket server
 * @param {http.Server} server - HTTP server instance
 */
function setupWebSocket(server) {
  const wss = new WebSocket.Server({ server, path: '/ws' });
  
  wss.on('connection', (ws, req) => {
    logger.info('WebSocket client connected', { ip: req.socket.remoteAddress });
    
    wsClients.add(ws);
    websocketConnections.inc();
    
    // Send welcome message
    ws.send(JSON.stringify({
      event: 'connected',
      message: 'Connected to notification service',
    }));
    
    // Handle client messages
    ws.on('message', (data) => {
      try {
        const message = JSON.parse(data);
        logger.info('WebSocket message received', { message });
        
        // Handle ping/pong for keepalive
        if (message.type === 'ping') {
          ws.send(JSON.stringify({ type: 'pong' }));
        }
      } catch (err) {
        logger.warn('Invalid WebSocket message', { error: err.message });
      }
    });
    
    // Handle disconnect
    ws.on('close', () => {
      wsClients.delete(ws);
      websocketConnections.dec();
      logger.info('WebSocket client disconnected');
    });
    
    // Handle errors
    ws.on('error', (err) => {
      logger.error('WebSocket error', { error: err.message });
      wsClients.delete(ws);
      websocketConnections.dec();
    });
  });
  
  logger.info('WebSocket server started');
}

// =============================================================================
// SERVER STARTUP
// =============================================================================

async function start() {
  logger.info('Starting Notification Service...');
  
  // Connect to Redis
  await connectRedis();
  
  // Connect to RabbitMQ
  await connectRabbitMQ();
  
  // Create HTTP server
  const server = http.createServer(app);
  
  // Setup WebSocket
  setupWebSocket(server);
  
  // Start listening
  server.listen(config.port, () => {
    logger.info(`Notification Service listening on port ${config.port}`);
  });
  
  // Graceful shutdown
  process.on('SIGTERM', async () => {
    logger.info('SIGTERM received, shutting down...');
    
    server.close(() => {
      logger.info('HTTP server closed');
    });
    
    if (rabbitChannel) await rabbitChannel.close();
    if (rabbitConnection) await rabbitConnection.close();
    if (redisClient) await redisClient.quit();
    
    process.exit(0);
  });
}

// Start the service
start().catch((err) => {
  logger.error('Failed to start service', { error: err.message });
  process.exit(1);
});
