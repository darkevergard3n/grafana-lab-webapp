#!/bin/bash
# =============================================================================
# DEPLOYMENT SCRIPT
# =============================================================================
# This script deploys the complete webapp stack.
#
# USAGE:
#   ./deploy.sh              # Deploy all services
#   ./deploy.sh --build      # Rebuild and deploy
#   ./deploy.sh --clean      # Clean up and redeploy
# =============================================================================

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Print with color
info() { echo -e "${GREEN}[INFO]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Check prerequisites
check_prerequisites() {
    info "Checking prerequisites..."
    
    if ! command -v docker &> /dev/null; then
        error "Docker is not installed. Please install Docker first."
        exit 1
    fi
    
    if ! docker compose version &> /dev/null; then
        error "Docker Compose V2 is not installed."
        exit 1
    fi
    
    info "Prerequisites OK!"
}

# Create .env file if not exists
setup_env() {
    if [ ! -f .env ]; then
        info "Creating .env file from template..."
        cp .env.example .env
        warn "Please review and update .env file with your settings!"
    fi
}

# Deploy services
deploy() {
    local build_flag=""
    
    if [ "$1" == "--build" ]; then
        build_flag="--build"
        info "Building images..."
    fi
    
    if [ "$1" == "--clean" ]; then
        info "Cleaning up existing containers..."
        docker compose down -v --remove-orphans
        build_flag="--build"
    fi
    
    info "Starting services..."
    docker compose up -d $build_flag
    
    info "Waiting for services to be healthy..."
    sleep 10
    
    # Check service health
    ./scripts/health-check.sh
}

# Main
main() {
    info "=== Order Management System Deployment ==="
    
    check_prerequisites
    setup_env
    deploy "$1"
    
    info ""
    info "=== Deployment Complete ==="
    info ""
    info "Access the services:"
    info "  Frontend:    http://localhost:3000"
    info "  Traefik:     http://localhost:8080"
    info "  RabbitMQ:    http://localhost:15672"
    info ""
    info "Run './scripts/health-check.sh' to verify all services."
}

main "$@"
