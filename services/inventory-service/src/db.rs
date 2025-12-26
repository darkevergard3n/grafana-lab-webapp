// =============================================================================
// DATABASE MODULE
// =============================================================================
// This module handles all PostgreSQL database operations.
//
// LEARNING NOTES:
// - SQLx provides compile-time checked SQL queries
// - Connection pooling improves performance
// - Transactions ensure data consistency
// =============================================================================

use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use uuid::Uuid;

use crate::models::{
    AdjustStockRequest, InventoryItem, LowStockAlert, ReleaseStockRequest,
    ReservationResponse, ReserveStockRequest,
};

// -----------------------------------------------------------------------------
// DATABASE WRAPPER
// -----------------------------------------------------------------------------
// This struct wraps the SQLx connection pool and provides typed methods
// for all database operations.
//
// LEARNING NOTE:
// Wrapping the pool in a struct allows us to:
// 1. Add custom methods for our domain operations
// 2. Hide the underlying SQLx types from the rest of the app
// 3. Easy testing with mock implementations
#[derive(Clone)]
pub struct Database {
    /// SQLx PostgreSQL connection pool
    /// PgPool manages multiple connections automatically
    pool: PgPool,
}

impl Database {
    // -------------------------------------------------------------------------
    // CONNECTION
    // -------------------------------------------------------------------------
    /// Create a new database connection pool
    ///
    /// # Arguments
    /// * `database_url` - PostgreSQL connection string
    ///
    /// # Returns
    /// * `Ok(Database)` - Connected database instance
    /// * `Err` - Connection failed
    ///
    /// # Example
    /// ```
    /// let db = Database::connect("postgres://user:pass@localhost/db").await?;
    /// ```
    pub async fn connect(database_url: &str) -> Result<Self> {
        // Create connection pool with sensible defaults
        let pool = PgPoolOptions::new()
            // Maximum number of connections in the pool
            // More connections = more concurrent queries, but more memory
            .max_connections(10)
            
            // Minimum connections to keep open (even when idle)
            .min_connections(2)
            
            // How long to wait for a connection before giving up
            .acquire_timeout(std::time::Duration::from_secs(5))
            
            // How long a connection can be idle before being closed
            .idle_timeout(std::time::Duration::from_secs(300))
            
            // Actually connect to the database
            .connect(database_url)
            .await
            .context("Failed to connect to PostgreSQL")?;

        Ok(Self { pool })
    }

    // -------------------------------------------------------------------------
    // MIGRATIONS
    // -------------------------------------------------------------------------
    /// Run database migrations to create/update tables
    ///
    /// This creates the inventory table if it doesn't exist and seeds
    /// initial data for testing.
    pub async fn run_migrations(&self) -> Result<()> {
        // Create the inventory table
        // IF NOT EXISTS ensures this is idempotent (safe to run multiple times)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS inventory (
                -- Primary key: UUID for global uniqueness
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                
                -- SKU must be unique (can't have duplicate products)
                sku VARCHAR(50) UNIQUE NOT NULL,
                
                -- Product name for display
                name VARCHAR(255) NOT NULL,
                
                -- Current stock quantity
                quantity INTEGER NOT NULL DEFAULT 0,
                
                -- Reserved stock (for pending orders)
                reserved INTEGER NOT NULL DEFAULT 0,
                
                -- Warehouse location code
                warehouse VARCHAR(50) NOT NULL DEFAULT 'DEFAULT',
                
                -- Alert threshold
                low_stock_threshold INTEGER NOT NULL DEFAULT 10,
                
                -- Timestamps for auditing
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                
                -- Ensure quantity is never negative
                CONSTRAINT positive_quantity CHECK (quantity >= 0),
                
                -- Ensure reserved doesn't exceed quantity
                CONSTRAINT valid_reserved CHECK (reserved >= 0 AND reserved <= quantity)
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Failed to create inventory table")?;

        // Create index on SKU for fast lookups
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_inventory_sku ON inventory(sku)
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Failed to create SKU index")?;

        // Create index on warehouse for filtering
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_inventory_warehouse ON inventory(warehouse)
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Failed to create warehouse index")?;

        // Seed sample data if table is empty
        self.seed_sample_data().await?;

        Ok(())
    }

    /// Seed sample inventory data for testing
    async fn seed_sample_data(&self) -> Result<()> {
        // Check if data already exists
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM inventory")
            .fetch_one(&self.pool)
            .await?;

        if count.0 > 0 {
            return Ok(()); // Data already exists
        }

        // Insert sample products
        let sample_items = vec![
            ("SKU-LAPTOP-001", "Dell XPS 15 Laptop", 50, "JKT-1", 10),
            ("SKU-LAPTOP-002", "MacBook Pro 14", 30, "JKT-1", 5),
            ("SKU-PHONE-001", "iPhone 15 Pro", 100, "JKT-1", 20),
            ("SKU-PHONE-002", "Samsung Galaxy S24", 75, "JKT-1", 15),
            ("SKU-TABLET-001", "iPad Pro 12.9", 40, "JKT-2", 8),
            ("SKU-MONITOR-001", "LG 27 4K Monitor", 25, "SBY-1", 5),
            ("SKU-KEYBOARD-001", "Logitech MX Keys", 200, "SBY-1", 30),
            ("SKU-MOUSE-001", "Logitech MX Master 3", 150, "SBY-1", 25),
            ("SKU-HEADPHONE-001", "Sony WH-1000XM5", 60, "JKT-2", 10),
            ("SKU-CABLE-001", "USB-C Cable 2m", 500, "JKT-1", 100),
        ];

        for (sku, name, quantity, warehouse, threshold) in sample_items {
            sqlx::query(
                r#"
                INSERT INTO inventory (sku, name, quantity, warehouse, low_stock_threshold)
                VALUES ($1, $2, $3, $4, $5)
                ON CONFLICT (sku) DO NOTHING
                "#,
            )
            .bind(sku)
            .bind(name)
            .bind(quantity)
            .bind(warehouse)
            .bind(threshold)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    // -------------------------------------------------------------------------
    // READ OPERATIONS
    // -------------------------------------------------------------------------

    /// Get all inventory items with pagination
    ///
    /// # Arguments
    /// * `page` - Page number (1-indexed)
    /// * `per_page` - Items per page
    ///
    /// # Returns
    /// Tuple of (items, total_count)
    pub async fn list_items(&self, page: i32, per_page: i32) -> Result<(Vec<InventoryItem>, i64)> {
        // Calculate offset for pagination
        // Page 1 = offset 0, Page 2 = offset per_page, etc.
        let offset = (page - 1) * per_page;

        // Get paginated items
        let items = sqlx::query_as::<_, InventoryItem>(
            r#"
            SELECT id, sku, name, quantity, reserved, warehouse, 
                   low_stock_threshold, created_at, updated_at
            FROM inventory
            ORDER BY sku ASC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(per_page)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch inventory items")?;

        // Get total count for pagination metadata
        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM inventory")
            .fetch_one(&self.pool)
            .await
            .context("Failed to count inventory items")?;

        Ok((items, total.0))
    }

    /// Get a single inventory item by SKU
    pub async fn get_by_sku(&self, sku: &str) -> Result<Option<InventoryItem>> {
        let item = sqlx::query_as::<_, InventoryItem>(
            r#"
            SELECT id, sku, name, quantity, reserved, warehouse,
                   low_stock_threshold, created_at, updated_at
            FROM inventory
            WHERE sku = $1
            "#,
        )
        .bind(sku)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch inventory item")?;

        Ok(item)
    }

    /// Get all items with low stock
    pub async fn get_low_stock_items(&self) -> Result<Vec<LowStockAlert>> {
        // Query items where available stock (quantity - reserved) < threshold
        let rows = sqlx::query(
            r#"
            SELECT sku, name, quantity - reserved as available, 
                   low_stock_threshold as threshold, warehouse
            FROM inventory
            WHERE (quantity - reserved) < low_stock_threshold
            ORDER BY (quantity - reserved) ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch low stock items")?;

        // Map rows to LowStockAlert structs
        let alerts = rows
            .iter()
            .map(|row| LowStockAlert {
                sku: row.get("sku"),
                name: row.get("name"),
                available: row.get("available"),
                threshold: row.get("threshold"),
                warehouse: row.get("warehouse"),
            })
            .collect();

        Ok(alerts)
    }

    // -------------------------------------------------------------------------
    // WRITE OPERATIONS
    // -------------------------------------------------------------------------

    /// Reserve stock for an order
    ///
    /// This atomically checks availability and reserves stock.
    /// Uses a transaction to ensure consistency.
    pub async fn reserve_stock(&self, req: &ReserveStockRequest) -> Result<ReservationResponse> {
        // Start a transaction
        // All operations inside will be atomic (all succeed or all fail)
        let mut tx = self.pool.begin().await?;

        // Lock the row for update to prevent race conditions
        // FOR UPDATE prevents other transactions from modifying this row
        let item = sqlx::query_as::<_, InventoryItem>(
            r#"
            SELECT id, sku, name, quantity, reserved, warehouse,
                   low_stock_threshold, created_at, updated_at
            FROM inventory
            WHERE sku = $1
            FOR UPDATE
            "#,
        )
        .bind(&req.sku)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| anyhow::anyhow!("SKU not found: {}", req.sku))?;

        // Check if enough stock is available
        let available = item.quantity - item.reserved;
        if available < req.quantity {
            return Err(anyhow::anyhow!(
                "Insufficient stock. Available: {}, Requested: {}",
                available,
                req.quantity
            ));
        }

        // Update reserved count
        sqlx::query(
            r#"
            UPDATE inventory
            SET reserved = reserved + $1, updated_at = NOW()
            WHERE sku = $2
            "#,
        )
        .bind(req.quantity)
        .bind(&req.sku)
        .execute(&mut *tx)
        .await?;

        // Commit the transaction
        tx.commit().await?;

        // Return reservation confirmation
        Ok(ReservationResponse {
            reservation_id: Uuid::new_v4(),
            sku: req.sku.clone(),
            quantity: req.quantity,
            created_at: Utc::now(),
            expires_at: Some(Utc::now() + chrono::Duration::hours(24)),
        })
    }

    /// Release previously reserved stock
    pub async fn release_stock(&self, req: &ReleaseStockRequest) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE inventory
            SET reserved = GREATEST(reserved - $1, 0), updated_at = NOW()
            WHERE sku = $2 AND reserved >= $1
            "#,
        )
        .bind(req.quantity)
        .bind(&req.sku)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(anyhow::anyhow!(
                "Failed to release stock. SKU not found or insufficient reserved quantity."
            ));
        }

        Ok(())
    }

    /// Adjust stock quantity (for manual corrections, receiving shipments, etc.)
    pub async fn adjust_stock(&self, req: &AdjustStockRequest) -> Result<InventoryItem> {
        let item = sqlx::query_as::<_, InventoryItem>(
            r#"
            UPDATE inventory
            SET quantity = GREATEST(quantity + $1, 0), updated_at = NOW()
            WHERE sku = $2
            RETURNING id, sku, name, quantity, reserved, warehouse,
                      low_stock_threshold, created_at, updated_at
            "#,
        )
        .bind(req.delta)
        .bind(&req.sku)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("SKU not found: {}", req.sku))?;

        Ok(item)
    }

    // -------------------------------------------------------------------------
    // HEALTH CHECK
    // -------------------------------------------------------------------------

    /// Check if database connection is healthy
    pub async fn health_check(&self) -> bool {
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .is_ok()
    }
}
