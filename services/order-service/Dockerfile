# =============================================================================
# DOCKERFILE - Go Order Service
# =============================================================================
# Multi-stage build for a small, secure production image.
#
# LEARNING NOTES:
# - Go compiles to a single static binary
# - We can use scratch or alpine as the final image
# - CGO_ENABLED=0 allows fully static compilation
# =============================================================================

# =============================================================================
# STAGE 1: BUILDER
# =============================================================================
FROM golang:1.21-alpine AS builder

# Install build dependencies
# git: Required for go mod download
# ca-certificates: For HTTPS connections
RUN apk add --no-cache git ca-certificates

# Set working directory
WORKDIR /app

# -----------------------------------------------------------------------------
# DEPENDENCY CACHING
# -----------------------------------------------------------------------------
# Copy go.mod and source files to generate go.sum
COPY go.mod ./
COPY . .

# Generate go.sum and download dependencies
RUN go mod tidy && go mod download

# -----------------------------------------------------------------------------
# BUILD APPLICATION
# -----------------------------------------------------------------------------

# Build the binary
# CGO_ENABLED=0: Disable C bindings for fully static binary
# -ldflags="-s -w": Strip debug info for smaller binary
# -o: Output file name
RUN CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build \
    -ldflags="-s -w" \
    -o order-service \
    .

# =============================================================================
# STAGE 2: RUNTIME
# =============================================================================
# Use minimal alpine image
FROM alpine:3.19

# Install runtime dependencies
RUN apk add --no-cache ca-certificates wget

# Create non-root user (using GID/UID 10000 to avoid conflicts)
RUN addgroup -g 10000 appgroup && \
    adduser -u 10000 -G appgroup -s /bin/sh -D appuser

# Copy binary from builder
COPY --from=builder /app/order-service /usr/local/bin/order-service

# Set ownership
RUN chown appuser:appgroup /usr/local/bin/order-service

# Switch to non-root user
USER appuser

# Document exposed port
EXPOSE 8001

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD wget --spider -q http://localhost:8001/health || exit 1

# Run the application
ENTRYPOINT ["order-service"]
