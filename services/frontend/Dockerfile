# =============================================================================
# DOCKERFILE - Next.js Frontend
# =============================================================================
FROM node:20-alpine AS builder

WORKDIR /app

# Copy package files
COPY package*.json ./

# Install dependencies
RUN npm install

# Copy source code
COPY . .

# Build the application
RUN npm run build

# =============================================================================
# PRODUCTION IMAGE
# =============================================================================
FROM node:20-alpine

WORKDIR /app

# Copy built application
COPY --from=builder /app/.next/standalone ./
COPY --from=builder /app/.next/static ./.next/static

# Set ownership to node user
RUN chown -R node:node /app

# Switch to non-root user (node user already exists in base image)
USER node

EXPOSE 3000

ENV PORT=3000
ENV HOSTNAME="0.0.0.0"

CMD ["node", "server.js"]
