// =============================================================================
// METRICS MODULE
// =============================================================================
// This module sets up Prometheus metrics for observability.
//
// LEARNING NOTES:
// - Prometheus uses a "pull" model - it scrapes /metrics endpoint
// - Metrics have types: Counter, Gauge, Histogram, Summary
// - Labels add dimensions to metrics (e.g., endpoint="/api/orders")
//
// METRIC TYPES EXPLAINED:
// - Counter: Only goes up (requests, errors). Resets on restart.
// - Gauge: Can go up or down (temperature, queue size, connections).
// - Histogram: Distribution of values in buckets (latency percentiles).
// - Summary: Like histogram but calculates percentiles client-side.
// =============================================================================

use anyhow::Result;
use metrics::{counter, gauge, histogram, describe_counter, describe_gauge, describe_histogram};
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};

// =============================================================================
// METRIC NAMES (Constants)
// =============================================================================
// Define metric names as constants to avoid typos and enable IDE autocomplete.
//
// NAMING CONVENTION (Prometheus best practices):
// - Use snake_case
// - Include unit in suffix: _seconds, _bytes, _total
// - Use _total suffix for counters
// - Be descriptive but not too long

/// HTTP request counter
/// Labels: method (GET/POST), endpoint (/api/v1/inventory), status (200/500)
pub const HTTP_REQUESTS_TOTAL: &str = "http_requests_total";

/// HTTP request duration histogram
/// Labels: method, endpoint
pub const HTTP_REQUEST_DURATION_SECONDS: &str = "http_request_duration_seconds";

/// Inventory stock level gauge
/// Labels: sku, warehouse
pub const INVENTORY_STOCK_LEVEL: &str = "inventory_stock_level";

/// Inventory reservations counter
/// Labels: sku, status (success/failed)
pub const INVENTORY_RESERVATIONS_TOTAL: &str = "inventory_reservations_total";

/// Low stock items gauge (current count of items below threshold)
pub const INVENTORY_LOW_STOCK_ITEMS: &str = "inventory_low_stock_items";

/// Database query duration histogram
/// Labels: operation (select/insert/update)
pub const DB_QUERY_DURATION_SECONDS: &str = "db_query_duration_seconds";

/// Redis operation duration histogram
/// Labels: operation (get/set/delete)
pub const REDIS_OPERATION_DURATION_SECONDS: &str = "redis_operation_duration_seconds";

// =============================================================================
// SETUP FUNCTION
// =============================================================================
/// Initialize Prometheus metrics recorder
///
/// This function:
/// 1. Creates a PrometheusBuilder
/// 2. Configures histogram buckets
/// 3. Installs the recorder globally
/// 4. Returns a handle for rendering metrics
///
/// # Returns
/// * `PrometheusHandle` - Used to render metrics in Prometheus format
///
/// # Example
/// ```
/// let handle = setup_metrics()?;
/// let metrics_output = handle.render();  // Returns Prometheus text format
/// ```
pub fn setup_metrics() -> Result<PrometheusHandle> {
    // -------------------------------------------------------------------------
    // HISTOGRAM BUCKETS
    // -------------------------------------------------------------------------
    // Histograms divide values into buckets. Each bucket counts how many
    // observations fell into that range.
    //
    // For latency metrics, we use buckets that make sense for HTTP requests:
    // - 1ms, 5ms, 10ms: Fast responses
    // - 25ms, 50ms, 100ms: Normal responses
    // - 250ms, 500ms, 1s: Slow responses
    // - 2.5s, 5s, 10s: Very slow (potential timeout)
    let latency_buckets = &[
        0.001,  // 1ms
        0.005,  // 5ms
        0.01,   // 10ms
        0.025,  // 25ms
        0.05,   // 50ms
        0.1,    // 100ms
        0.25,   // 250ms
        0.5,    // 500ms
        1.0,    // 1 second
        2.5,    // 2.5 seconds
        5.0,    // 5 seconds
        10.0,   // 10 seconds
    ];

    // Build the Prometheus exporter
    let handle = PrometheusBuilder::new()
        // Configure buckets for HTTP request duration
        .set_buckets_for_metric(
            Matcher::Full(HTTP_REQUEST_DURATION_SECONDS.to_string()),
            latency_buckets,
        )?
        // Configure buckets for database queries
        .set_buckets_for_metric(
            Matcher::Full(DB_QUERY_DURATION_SECONDS.to_string()),
            latency_buckets,
        )?
        // Configure buckets for Redis operations
        .set_buckets_for_metric(
            Matcher::Full(REDIS_OPERATION_DURATION_SECONDS.to_string()),
            latency_buckets,
        )?
        // Install as the global metrics recorder
        .install_recorder()?;

    // -------------------------------------------------------------------------
    // METRIC DESCRIPTIONS
    // -------------------------------------------------------------------------
    // Descriptions appear in the /metrics output as HELP comments.
    // They help humans understand what each metric measures.

    describe_counter!(
        HTTP_REQUESTS_TOTAL,
        "Total number of HTTP requests received"
    );

    describe_histogram!(
        HTTP_REQUEST_DURATION_SECONDS,
        "HTTP request latency in seconds"
    );

    describe_gauge!(
        INVENTORY_STOCK_LEVEL,
        "Current stock level for each SKU"
    );

    describe_counter!(
        INVENTORY_RESERVATIONS_TOTAL,
        "Total number of stock reservation attempts"
    );

    describe_gauge!(
        INVENTORY_LOW_STOCK_ITEMS,
        "Number of items currently below low stock threshold"
    );

    describe_histogram!(
        DB_QUERY_DURATION_SECONDS,
        "Database query latency in seconds"
    );

    describe_histogram!(
        REDIS_OPERATION_DURATION_SECONDS,
        "Redis operation latency in seconds"
    );

    Ok(handle)
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================
// These functions provide a convenient API for recording metrics.
// They wrap the raw metrics macros with proper labels.

/// Record an HTTP request
///
/// # Arguments
/// * `method` - HTTP method (GET, POST, etc.)
/// * `endpoint` - Request path (/api/v1/inventory)
/// * `status` - Response status code (200, 404, 500)
/// * `duration_secs` - Request duration in seconds
pub fn record_http_request(method: &str, endpoint: &str, status: u16, duration_secs: f64) {
    // Increment request counter
    counter!(
        HTTP_REQUESTS_TOTAL,
        "method" => method.to_string(),
        "endpoint" => endpoint.to_string(),
        "status" => status.to_string()
    )
    .increment(1);

    // Record latency in histogram
    histogram!(
        HTTP_REQUEST_DURATION_SECONDS,
        "method" => method.to_string(),
        "endpoint" => endpoint.to_string()
    )
    .record(duration_secs);
}

/// Update stock level gauge for a SKU
///
/// # Arguments
/// * `sku` - Stock Keeping Unit identifier
/// * `warehouse` - Warehouse code
/// * `level` - Current stock level
pub fn set_stock_level(sku: &str, warehouse: &str, level: i32) {
    gauge!(
        INVENTORY_STOCK_LEVEL,
        "sku" => sku.to_string(),
        "warehouse" => warehouse.to_string()
    )
    .set(level as f64);
}

/// Record a stock reservation attempt
///
/// # Arguments
/// * `sku` - Stock Keeping Unit identifier
/// * `success` - Whether the reservation succeeded
pub fn record_reservation(sku: &str, success: bool) {
    let status = if success { "success" } else { "failed" };
    counter!(
        INVENTORY_RESERVATIONS_TOTAL,
        "sku" => sku.to_string(),
        "status" => status.to_string()
    )
    .increment(1);
}

/// Update low stock items count
///
/// # Arguments
/// * `count` - Number of items below threshold
pub fn set_low_stock_count(count: i64) {
    gauge!(INVENTORY_LOW_STOCK_ITEMS).set(count as f64);
}

/// Record database query duration
///
/// # Arguments
/// * `operation` - Type of operation (select, insert, update)
/// * `duration_secs` - Query duration in seconds
pub fn record_db_query(operation: &str, duration_secs: f64) {
    histogram!(
        DB_QUERY_DURATION_SECONDS,
        "operation" => operation.to_string()
    )
    .record(duration_secs);
}

/// Record Redis operation duration
///
/// # Arguments
/// * `operation` - Type of operation (get, set, delete)
/// * `duration_secs` - Operation duration in seconds
pub fn record_redis_operation(operation: &str, duration_secs: f64) {
    histogram!(
        REDIS_OPERATION_DURATION_SECONDS,
        "operation" => operation.to_string()
    )
    .record(duration_secs);
}
