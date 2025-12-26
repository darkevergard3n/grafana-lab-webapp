// =============================================================================
// MODELS MODULE
// =============================================================================
// This module defines the data structures used throughout the service.
//
// LEARNING NOTES:
// - Rust uses structs to define data structures
// - Derive macros automatically implement common traits
// - Serde handles JSON serialization/deserialization
// =============================================================================

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// =============================================================================
// INVENTORY ITEM
// =============================================================================
// Represents a single inventory item (product) in the system.
//
// DERIVE MACROS EXPLAINED:
// - Debug: Allows printing with {:?} for debugging
// - Clone: Allows creating copies of the struct
// - Serialize: Converts struct to JSON (for API responses)
// - Deserialize: Converts JSON to struct (for API requests)
// - FromRow: Allows SQLx to map database rows to this struct
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct InventoryItem {
    /// Unique identifier for the inventory record
    /// UUID v4 is randomly generated and globally unique
    pub id: Uuid,
    
    /// Stock Keeping Unit - unique product identifier
    /// Example: "SKU-12345", "LAPTOP-DELL-001"
    pub sku: String,
    
    /// Human-readable product name
    pub name: String,
    
    /// Current quantity in stock
    /// Can be negative if we allow backorders (not recommended)
    pub quantity: i32,
    
    /// Quantity reserved for pending orders
    /// Reserved stock can't be sold to other customers
    pub reserved: i32,
    
    /// Warehouse location code
    /// Example: "JKT-1" (Jakarta Warehouse 1)
    pub warehouse: String,
    
    /// Minimum stock level before triggering low stock alert
    pub low_stock_threshold: i32,
    
    /// When this record was created
    pub created_at: DateTime<Utc>,
    
    /// When this record was last updated
    pub updated_at: DateTime<Utc>,
}

// -----------------------------------------------------------------------------
// COMPUTED PROPERTIES (impl block)
// -----------------------------------------------------------------------------
// In Rust, we add methods to structs using `impl` blocks.
// These methods can compute derived values or perform operations.
impl InventoryItem {
    /// Calculate available stock (total - reserved)
    /// This is what can actually be sold to new customers
    /// 
    /// # Example
    /// ```
    /// let item = InventoryItem { quantity: 100, reserved: 25, ... };
    /// assert_eq!(item.available(), 75);
    /// ```
    pub fn available(&self) -> i32 {
        self.quantity - self.reserved
    }
    
    /// Check if stock is below the low stock threshold
    pub fn is_low_stock(&self) -> bool {
        self.available() < self.low_stock_threshold
    }
}

// =============================================================================
// API REQUEST/RESPONSE STRUCTURES
// =============================================================================
// These structs define the shape of API requests and responses.
// Separating these from database models is a good practice because:
// 1. API shape can change without changing database schema
// 2. We can hide internal fields from API consumers
// 3. We can validate input separately from database constraints

// -----------------------------------------------------------------------------
// STOCK RESERVATION REQUEST
// -----------------------------------------------------------------------------
/// Request body for reserving stock
/// 
/// # Example JSON
/// ```json
/// {
///   "sku": "LAPTOP-001",
///   "quantity": 5,
///   "order_id": "ORD-12345"
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReserveStockRequest {
    /// SKU of the product to reserve
    pub sku: String,
    
    /// Quantity to reserve
    pub quantity: i32,
    
    /// Order ID this reservation is for (for tracking)
    pub order_id: String,
}

// -----------------------------------------------------------------------------
// STOCK RELEASE REQUEST
// -----------------------------------------------------------------------------
/// Request body for releasing reserved stock
/// Used when an order is cancelled or expired
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseStockRequest {
    /// SKU of the product
    pub sku: String,
    
    /// Quantity to release back to available stock
    pub quantity: i32,
    
    /// Original order ID
    pub order_id: String,
}

// -----------------------------------------------------------------------------
// STOCK ADJUSTMENT REQUEST
// -----------------------------------------------------------------------------
/// Request body for manual stock adjustments
/// Used for inventory corrections, receiving shipments, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdjustStockRequest {
    /// SKU of the product
    pub sku: String,
    
    /// Amount to adjust (positive to add, negative to remove)
    pub delta: i32,
    
    /// Reason for adjustment (for audit trail)
    pub reason: String,
}

// -----------------------------------------------------------------------------
// RESERVATION RESPONSE
// -----------------------------------------------------------------------------
/// Response after successfully reserving stock
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReservationResponse {
    /// Unique ID for this reservation
    pub reservation_id: Uuid,
    
    /// SKU that was reserved
    pub sku: String,
    
    /// Quantity reserved
    pub quantity: i32,
    
    /// When the reservation was made
    pub created_at: DateTime<Utc>,
    
    /// When the reservation expires (optional)
    pub expires_at: Option<DateTime<Utc>>,
}

// -----------------------------------------------------------------------------
// INVENTORY LIST RESPONSE
// -----------------------------------------------------------------------------
/// Response for listing inventory items with pagination metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryListResponse {
    /// List of inventory items
    pub items: Vec<InventoryItem>,
    
    /// Total count (for pagination)
    pub total: i64,
    
    /// Current page number
    pub page: i32,
    
    /// Items per page
    pub per_page: i32,
}

// -----------------------------------------------------------------------------
// LOW STOCK ALERT
// -----------------------------------------------------------------------------
/// Represents a low stock alert for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LowStockAlert {
    /// Product SKU
    pub sku: String,
    
    /// Product name
    pub name: String,
    
    /// Current available quantity
    pub available: i32,
    
    /// Configured threshold
    pub threshold: i32,
    
    /// Warehouse location
    pub warehouse: String,
}

// =============================================================================
// HEALTH CHECK RESPONSES
// =============================================================================
// Standard health check response structures

/// Simple health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub version: String,
}

/// Detailed readiness check response
#[derive(Debug, Serialize)]
pub struct ReadinessResponse {
    pub status: String,
    pub checks: ReadinessChecks,
}

/// Individual dependency health checks
#[derive(Debug, Serialize)]
pub struct ReadinessChecks {
    pub database: bool,
    pub redis: bool,
}

// =============================================================================
// ERROR RESPONSES
// =============================================================================
// Standardized error response format for API

/// API error response body
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    /// Error type/code
    pub error: String,
    
    /// Human-readable error message
    pub message: String,
    
    /// Optional additional details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

impl ErrorResponse {
    /// Create a new error response
    pub fn new(error: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            message: message.into(),
            details: None,
        }
    }
    
    /// Create an error response with details
    pub fn with_details(
        error: impl Into<String>,
        message: impl Into<String>,
        details: impl Into<String>,
    ) -> Self {
        Self {
            error: error.into(),
            message: message.into(),
            details: Some(details.into()),
        }
    }
}
