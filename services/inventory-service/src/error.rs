// =============================================================================
// ERROR MODULE
// =============================================================================
// This module defines custom error types and their HTTP responses.
//
// LEARNING NOTES:
// - Rust doesn't have exceptions; it uses Result<T, E> for error handling
// - thiserror crate makes defining error types easy
// - We convert our errors to HTTP responses using Axum's IntoResponse
//
// ERROR HANDLING PHILOSOPHY:
// - Errors should be informative but not leak internal details
// - Use typed errors instead of stringly-typed errors
// - Map errors to appropriate HTTP status codes
// =============================================================================

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use thiserror::Error;

use crate::models::ErrorResponse;

// =============================================================================
// CUSTOM ERROR TYPE
// =============================================================================
// This enum defines all possible errors in our service.
//
// LEARNING NOTE:
// The #[error("...")] attribute from thiserror automatically implements
// Display trait, so we get nice error messages for free.
//
// The #[from] attribute auto-implements From<X> for conversion.
#[derive(Debug, Error)]
pub enum AppError {
    // -------------------------------------------------------------------------
    // DATABASE ERRORS
    // -------------------------------------------------------------------------
    /// Database query failed
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    // -------------------------------------------------------------------------
    // REDIS ERRORS
    // -------------------------------------------------------------------------
    /// Redis operation failed
    #[error("Cache error: {0}")]
    Redis(#[from] redis::RedisError),

    // -------------------------------------------------------------------------
    // BUSINESS LOGIC ERRORS
    // -------------------------------------------------------------------------
    /// Item not found in inventory
    #[error("Item not found: {0}")]
    NotFound(String),

    /// Insufficient stock for operation
    #[error("Insufficient stock: available {available}, requested {requested}")]
    InsufficientStock { available: i32, requested: i32 },

    /// Invalid request data
    #[error("Invalid request: {0}")]
    BadRequest(String),

    // -------------------------------------------------------------------------
    // INTERNAL ERRORS
    // -------------------------------------------------------------------------
    /// Generic internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

// =============================================================================
// HTTP RESPONSE CONVERSION
// =============================================================================
// Axum uses the IntoResponse trait to convert types into HTTP responses.
// By implementing this for AppError, we can return errors directly from handlers.
//
// LEARNING NOTE:
// This pattern allows clean handler code like:
//   async fn handler() -> Result<Json<T>, AppError> { ... }
// Errors are automatically converted to proper HTTP responses.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // Determine HTTP status code based on error type
        let (status, error_code, message) = match &self {
            // 404 Not Found: Resource doesn't exist
            AppError::NotFound(msg) => (
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                msg.clone(),
            ),

            // 400 Bad Request: Client sent invalid data
            AppError::BadRequest(msg) => (
                StatusCode::BAD_REQUEST,
                "BAD_REQUEST",
                msg.clone(),
            ),

            // 409 Conflict: Business rule violation (not enough stock)
            AppError::InsufficientStock { available, requested } => (
                StatusCode::CONFLICT,
                "INSUFFICIENT_STOCK",
                format!("Available: {}, Requested: {}", available, requested),
            ),

            // 500 Internal Server Error: Something went wrong on our side
            // IMPORTANT: Don't expose internal details in production!
            AppError::Database(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "DATABASE_ERROR",
                "A database error occurred".to_string(),
            ),

            AppError::Redis(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "CACHE_ERROR",
                "A cache error occurred".to_string(),
            ),

            AppError::Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                msg.clone(),
            ),
        };

        // Log the error for debugging
        // In production, this goes to your logging system (Loki)
        tracing::error!(
            error_code = error_code,
            message = %message,
            "Request failed"
        );

        // Build the JSON response body
        let body = ErrorResponse::new(error_code, message);

        // Combine status code and body into a response
        (status, Json(body)).into_response()
    }
}

// =============================================================================
// RESULT TYPE ALIAS
// =============================================================================
// A convenient type alias for Results that use our error type.
// This saves typing Result<T, AppError> everywhere.
pub type AppResult<T> = Result<T, AppError>;

// =============================================================================
// CONVERSION HELPERS
// =============================================================================
// Sometimes we need to convert between error types.

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::Internal(err.to_string())
    }
}
