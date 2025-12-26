// =============================================================================
// HANDLERS MODULE
// =============================================================================
// This module contains all HTTP request handlers (controller layer).
//
// LEARNING NOTES:
// - Handlers are async functions that receive requests and return responses
// - Axum uses "extractors" to parse request data (path params, JSON body, etc.)
// - State is shared via the State<T> extractor
//
// AXUM EXTRACTORS EXPLAINED:
// - State<T>: Access shared application state
// - Path<T>: Extract path parameters (/items/:id → id)
// - Query<T>: Extract query parameters (?page=1 → page)
// - Json<T>: Parse JSON request body
// =============================================================================

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use std::time::Instant;

use crate::error::{AppError, AppResult};
use crate::metrics;
use crate::models::*;
use crate::AppState;

// =============================================================================
// HEALTH CHECK ENDPOINTS
// =============================================================================
// These endpoints are used by orchestrators (Kubernetes, Docker) to determine
// if the service is running and ready to receive traffic.

/// Liveness probe - Is the service running?
///
/// Returns 200 OK if the service is alive.
/// If this fails, the orchestrator will restart the container.
///
/// GET /health
pub async fn health_check() -> Json<HealthResponse> {
    // Simply return OK - if we can respond, we're alive
    Json(HealthResponse {
        status: "ok".to_string(),
        service: "inventory-service".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Readiness probe - Is the service ready to handle requests?
///
/// Checks if dependencies (database, Redis) are accessible.
/// If this fails, the orchestrator won't send traffic to this instance.
///
/// GET /ready
pub async fn readiness_check(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ReadinessResponse>, StatusCode> {
    // Check database connectivity
    let db_healthy = state.db.health_check().await;

    // Check Redis connectivity
    let redis_healthy = redis::cmd("PING")
        .query_async::<_, String>(&mut state.redis.clone())
        .await
        .is_ok();

    // Determine overall status
    let all_healthy = db_healthy && redis_healthy;
    let status = if all_healthy { "ready" } else { "not_ready" };

    let response = ReadinessResponse {
        status: status.to_string(),
        checks: ReadinessChecks {
            database: db_healthy,
            redis: redis_healthy,
        },
    };

    if all_healthy {
        Ok(Json(response))
    } else {
        // Return 503 Service Unavailable if not ready
        Err(StatusCode::SERVICE_UNAVAILABLE)
    }
}

// =============================================================================
// METRICS ENDPOINT
// =============================================================================
/// Prometheus metrics endpoint
///
/// Returns all metrics in Prometheus text format.
/// Prometheus server scrapes this endpoint periodically.
///
/// GET /metrics
///
/// # Example Response
/// ```
/// # HELP http_requests_total Total number of HTTP requests
/// # TYPE http_requests_total counter
/// http_requests_total{method="GET",endpoint="/api/v1/inventory",status="200"} 42
/// ```
pub async fn metrics_handler(State(state): State<Arc<AppState>>) -> String {
    // Render all metrics in Prometheus exposition format
    state.metrics_handle.render()
}

// =============================================================================
// INVENTORY API ENDPOINTS
// =============================================================================

// -----------------------------------------------------------------------------
// QUERY PARAMETERS
// -----------------------------------------------------------------------------
/// Query parameters for list endpoint
///
/// # Example
/// GET /api/v1/inventory?page=2&per_page=20
#[derive(Debug, Deserialize)]
pub struct ListParams {
    /// Page number (1-indexed, default: 1)
    #[serde(default = "default_page")]
    pub page: i32,

    /// Items per page (default: 20, max: 100)
    #[serde(default = "default_per_page")]
    pub per_page: i32,
}

fn default_page() -> i32 {
    1
}
fn default_per_page() -> i32 {
    20
}

// -----------------------------------------------------------------------------
// LIST INVENTORY
// -----------------------------------------------------------------------------
/// List all inventory items with pagination
///
/// GET /api/v1/inventory
/// GET /api/v1/inventory?page=2&per_page=50
///
/// # Query Parameters
/// - `page`: Page number (default: 1)
/// - `per_page`: Items per page (default: 20, max: 100)
///
/// # Response
/// ```json
/// {
///   "items": [...],
///   "total": 150,
///   "page": 1,
///   "per_page": 20
/// }
/// ```
pub async fn list_inventory(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListParams>,
) -> AppResult<Json<InventoryListResponse>> {
    // Start timing for metrics
    let start = Instant::now();

    // Validate pagination parameters
    let page = params.page.max(1); // Minimum page is 1
    let per_page = params.per_page.clamp(1, 100); // Between 1 and 100

    // Fetch items from database
    let (items, total) = state.db.list_items(page, per_page).await?;

    // Record metrics
    let duration = start.elapsed().as_secs_f64();
    metrics::record_http_request("GET", "/api/v1/inventory", 200, duration);
    metrics::record_db_query("select", duration);

    // Update stock level gauges for each item
    for item in &items {
        metrics::set_stock_level(&item.sku, &item.warehouse, item.available());
    }

    Ok(Json(InventoryListResponse {
        items,
        total,
        page,
        per_page,
    }))
}

// -----------------------------------------------------------------------------
// GET SINGLE ITEM
// -----------------------------------------------------------------------------
/// Get a single inventory item by SKU
///
/// GET /api/v1/inventory/:sku
///
/// # Path Parameters
/// - `sku`: Stock Keeping Unit identifier
///
/// # Response
/// - 200 OK: Item found, returns item JSON
/// - 404 Not Found: Item doesn't exist
pub async fn get_item(
    State(state): State<Arc<AppState>>,
    Path(sku): Path<String>,
) -> AppResult<Json<InventoryItem>> {
    let start = Instant::now();

    // Try to get from cache first (Redis)
    let cache_key = format!("inventory:{}", sku);
    let cached: Option<String> = redis::cmd("GET")
        .arg(&cache_key)
        .query_async(&mut state.redis.clone())
        .await
        .ok();

    if let Some(cached_json) = cached {
        // Cache hit! Parse and return
        if let Ok(item) = serde_json::from_str::<InventoryItem>(&cached_json) {
            let duration = start.elapsed().as_secs_f64();
            metrics::record_http_request("GET", "/api/v1/inventory/:sku", 200, duration);
            metrics::record_redis_operation("get", duration);
            return Ok(Json(item));
        }
    }

    // Cache miss - fetch from database
    let item = state
        .db
        .get_by_sku(&sku)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("SKU not found: {}", sku)))?;

    // Store in cache for 5 minutes
    let item_json = serde_json::to_string(&item).unwrap_or_default();
    let _: Result<(), _> = redis::cmd("SETEX")
        .arg(&cache_key)
        .arg(300) // 5 minutes TTL
        .arg(&item_json)
        .query_async(&mut state.redis.clone())
        .await;

    let duration = start.elapsed().as_secs_f64();
    metrics::record_http_request("GET", "/api/v1/inventory/:sku", 200, duration);
    metrics::record_db_query("select", duration);

    Ok(Json(item))
}

// -----------------------------------------------------------------------------
// RESERVE STOCK
// -----------------------------------------------------------------------------
/// Reserve stock for an order
///
/// POST /api/v1/inventory/reserve
///
/// # Request Body
/// ```json
/// {
///   "sku": "SKU-LAPTOP-001",
///   "quantity": 5,
///   "order_id": "ORD-12345"
/// }
/// ```
///
/// # Response
/// - 200 OK: Stock reserved successfully
/// - 409 Conflict: Insufficient stock
/// - 404 Not Found: SKU doesn't exist
pub async fn reserve_stock(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ReserveStockRequest>,
) -> AppResult<Json<ReservationResponse>> {
    let start = Instant::now();

    // Log the reservation attempt
    tracing::info!(
        sku = %request.sku,
        quantity = request.quantity,
        order_id = %request.order_id,
        "Attempting to reserve stock"
    );

    // Perform the reservation
    let result = state.db.reserve_stock(&request).await;

    let duration = start.elapsed().as_secs_f64();

    match result {
        Ok(reservation) => {
            // Success - record metrics
            metrics::record_http_request("POST", "/api/v1/inventory/reserve", 200, duration);
            metrics::record_reservation(&request.sku, true);

            // Invalidate cache for this SKU
            let cache_key = format!("inventory:{}", request.sku);
            let _: Result<(), _> = redis::cmd("DEL")
                .arg(&cache_key)
                .query_async(&mut state.redis.clone())
                .await;

            tracing::info!(
                reservation_id = %reservation.reservation_id,
                "Stock reserved successfully"
            );

            Ok(Json(reservation))
        }
        Err(e) => {
            // Failure - record metrics and return error
            metrics::record_http_request("POST", "/api/v1/inventory/reserve", 409, duration);
            metrics::record_reservation(&request.sku, false);

            tracing::warn!(
                sku = %request.sku,
                error = %e,
                "Failed to reserve stock"
            );

            Err(AppError::BadRequest(e.to_string()))
        }
    }
}

// -----------------------------------------------------------------------------
// RELEASE STOCK
// -----------------------------------------------------------------------------
/// Release previously reserved stock
///
/// POST /api/v1/inventory/release
///
/// Called when an order is cancelled or expires.
///
/// # Request Body
/// ```json
/// {
///   "sku": "SKU-LAPTOP-001",
///   "quantity": 5,
///   "order_id": "ORD-12345"
/// }
/// ```
pub async fn release_stock(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ReleaseStockRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let start = Instant::now();

    tracing::info!(
        sku = %request.sku,
        quantity = request.quantity,
        order_id = %request.order_id,
        "Releasing reserved stock"
    );

    state.db.release_stock(&request).await?;

    // Invalidate cache
    let cache_key = format!("inventory:{}", request.sku);
    let _: Result<(), _> = redis::cmd("DEL")
        .arg(&cache_key)
        .query_async(&mut state.redis.clone())
        .await;

    let duration = start.elapsed().as_secs_f64();
    metrics::record_http_request("POST", "/api/v1/inventory/release", 200, duration);

    Ok(Json(serde_json::json!({
        "status": "released",
        "sku": request.sku,
        "quantity": request.quantity
    })))
}

// -----------------------------------------------------------------------------
// ADJUST STOCK
// -----------------------------------------------------------------------------
/// Manually adjust stock quantity
///
/// POST /api/v1/inventory/adjust
///
/// Used for inventory corrections, receiving shipments, etc.
///
/// # Request Body
/// ```json
/// {
///   "sku": "SKU-LAPTOP-001",
///   "delta": 10,
///   "reason": "Received shipment from supplier"
/// }
/// ```
pub async fn adjust_stock(
    State(state): State<Arc<AppState>>,
    Json(request): Json<AdjustStockRequest>,
) -> AppResult<Json<InventoryItem>> {
    let start = Instant::now();

    tracing::info!(
        sku = %request.sku,
        delta = request.delta,
        reason = %request.reason,
        "Adjusting stock"
    );

    let item = state.db.adjust_stock(&request).await?;

    // Update metrics
    metrics::set_stock_level(&item.sku, &item.warehouse, item.available());

    // Invalidate cache
    let cache_key = format!("inventory:{}", request.sku);
    let _: Result<(), _> = redis::cmd("DEL")
        .arg(&cache_key)
        .query_async(&mut state.redis.clone())
        .await;

    let duration = start.elapsed().as_secs_f64();
    metrics::record_http_request("POST", "/api/v1/inventory/adjust", 200, duration);

    Ok(Json(item))
}

// -----------------------------------------------------------------------------
// LOW STOCK ALERTS
// -----------------------------------------------------------------------------
/// Get all items with low stock
///
/// GET /api/v1/inventory/alerts
///
/// Returns items where available stock is below the threshold.
pub async fn low_stock_alerts(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<Vec<LowStockAlert>>> {
    let start = Instant::now();

    let alerts = state.db.get_low_stock_items().await?;

    // Update low stock count metric
    metrics::set_low_stock_count(alerts.len() as i64);

    let duration = start.elapsed().as_secs_f64();
    metrics::record_http_request("GET", "/api/v1/inventory/alerts", 200, duration);

    Ok(Json(alerts))
}
