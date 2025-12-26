# =============================================================================
# DOCKERFILE - Python Payment Service
# =============================================================================
# Multi-stage build for a secure production image.
#
# LEARNING NOTES:
# - Python images can be large; we use slim variants
# - Virtual environments isolate dependencies
# - Non-root user for security
# =============================================================================

# =============================================================================
# STAGE 1: BUILDER
# =============================================================================
FROM python:3.12-slim AS builder

# Set environment variables
# PYTHONDONTWRITEBYTECODE: Prevent .pyc files
# PYTHONUNBUFFERED: Force stdout/stderr to be unbuffered
ENV PYTHONDONTWRITEBYTECODE=1 \
    PYTHONUNBUFFERED=1 \
    PIP_NO_CACHE_DIR=1 \
    PIP_DISABLE_PIP_VERSION_CHECK=1

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Create virtual environment
RUN python -m venv /opt/venv
ENV PATH="/opt/venv/bin:$PATH"

# Install Python dependencies
COPY requirements.txt .
RUN pip install --upgrade pip && \
    pip install -r requirements.txt

# =============================================================================
# STAGE 2: RUNTIME
# =============================================================================
FROM python:3.12-slim

# Set environment variables
ENV PYTHONDONTWRITEBYTECODE=1 \
    PYTHONUNBUFFERED=1 \
    PATH="/opt/venv/bin:$PATH"

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user (using GID/UID 10000 to avoid conflicts)
RUN groupadd -g 10000 appgroup && \
    useradd -u 10000 -g appgroup -s /bin/bash -m appuser

# Copy virtual environment from builder
COPY --from=builder /opt/venv /opt/venv

# Copy application code
COPY --chown=appuser:appgroup . .

# Switch to non-root user
USER appuser

# Document exposed port
EXPOSE 8003

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8003/health || exit 1

# Run the application
CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8003"]
