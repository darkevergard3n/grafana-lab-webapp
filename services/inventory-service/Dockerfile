# =============================================================================
# DOCKERFILE - Rust Inventory Service
# =============================================================================
# This Dockerfile builds the Rust inventory service using a multi-stage build.
#
# MULTI-STAGE BUILD EXPLAINED:
# Stage 1 (builder): Compiles the Rust code with all build tools
# Stage 2 (runtime): Contains only the compiled binary (much smaller)
#
# BENEFITS:
# - Final image is ~20MB instead of ~1GB
# - No build tools in production (security)
# - Faster deployment (smaller image to pull)
# =============================================================================

# =============================================================================
# STAGE 1: BUILDER
# =============================================================================
# Use the official Rust image with Alpine Linux for smaller size
# Alpine is a minimal Linux distribution (~5MB base)
FROM rust:alpine AS builder

# Install build dependencies
# musl-dev: C library for static linking
# pkgconfig, openssl-dev: Required for some Rust crates
RUN apk add --no-cache \
    musl-dev \
    pkgconfig \
    openssl-dev \
    openssl-libs-static

# Create a new directory for our application
WORKDIR /app

# -----------------------------------------------------------------------------
# DEPENDENCY CACHING & BUILD
# -----------------------------------------------------------------------------
# Copy dependency manifest and source code
COPY Cargo.toml ./
COPY src ./src

# Build the application
RUN cargo build --release

# =============================================================================
# STAGE 2: RUNTIME
# =============================================================================
# Use a minimal Alpine image for the final container
FROM alpine:3.19

# Install runtime dependencies
# ca-certificates: For HTTPS connections
# libgcc: Required by some Rust programs
RUN apk add --no-cache \
    ca-certificates \
    libgcc

# Create a non-root user for security (using GID/UID 10000 to avoid conflicts)
# Running as root in containers is a security risk
RUN addgroup -g 10000 appgroup && \
    adduser -u 10000 -G appgroup -s /bin/sh -D appuser

# Copy the compiled binary from builder stage
COPY --from=builder /app/target/release/inventory-service /usr/local/bin/inventory-service

# Change ownership to non-root user
RUN chown appuser:appgroup /usr/local/bin/inventory-service

# Switch to non-root user
USER appuser

# Document which port the service uses
# This is informational; actual port binding is done at runtime
EXPOSE 8002

# Health check for container orchestration
# Docker/Kubernetes will use this to determine if container is healthy
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD wget --spider -q http://localhost:8002/health || exit 1

# Set the entrypoint to our application
# Using exec form (JSON array) for proper signal handling
ENTRYPOINT ["inventory-service"]
