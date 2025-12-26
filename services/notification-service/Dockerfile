# =============================================================================
# DOCKERFILE - Node.js Notification Service
# =============================================================================
FROM node:20-alpine

# Create app directory
WORKDIR /app

# Copy package files
COPY package*.json ./

# Install dependencies
RUN npm install --omit=dev

# Copy source code
COPY . .

# Switch to non-root user (node user already exists in base image)
USER node

# Expose port
EXPOSE 8005

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD wget --spider -q http://localhost:8005/health || exit 1

# Start the application
CMD ["node", "index.js"]
