# =============================================================================
# DOCKERFILE - Java User Service (Spring Boot)
# =============================================================================
# Multi-stage build for a production-ready Java container.
#
# LEARNING NOTES:
# - Uses Eclipse Temurin (formerly AdoptOpenJDK)
# - Multi-stage build reduces image size
# - Spring Boot creates an executable JAR (fat JAR)
# - JVM tuning is important for containers
# =============================================================================

# =============================================================================
# STAGE 1: BUILD
# =============================================================================
FROM eclipse-temurin:21-jdk-alpine AS builder

WORKDIR /app

# Install Maven
RUN apk add --no-cache maven

# Copy pom.xml first for dependency caching
COPY pom.xml .

# Download dependencies (cached if pom.xml unchanged)
RUN mvn dependency:go-offline -B

# Copy source code
COPY src ./src

# Build the application
# -DskipTests: Skip tests during build
# -Dmaven.javadoc.skip=true: Skip JavaDoc generation
RUN mvn package -DskipTests -Dmaven.javadoc.skip=true

# =============================================================================
# STAGE 2: RUNTIME
# =============================================================================
FROM eclipse-temurin:21-jre-alpine

WORKDIR /app

# Install curl for health checks
RUN apk add --no-cache curl

# Create non-root user (using GID/UID 10000 to avoid conflicts)
RUN addgroup -g 10000 appgroup && \
    adduser -u 10000 -G appgroup -s /bin/sh -D appuser

# Copy JAR from builder stage
COPY --from=builder /app/target/*.jar app.jar

# Set ownership
RUN chown -R appuser:appgroup /app

# Switch to non-root user
USER appuser

# Document exposed port
EXPOSE 8004

# JVM tuning for containers
# -XX:+UseContainerSupport: Use container memory limits
# -XX:MaxRAMPercentage=75: Use 75% of available memory
# -XX:+UseG1GC: Use G1 garbage collector
ENV JAVA_OPTS="-XX:+UseContainerSupport -XX:MaxRAMPercentage=75.0 -XX:+UseG1GC"

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
    CMD curl -f http://localhost:8004/health || exit 1

# Run the application
# Using shell form to expand JAVA_OPTS
ENTRYPOINT ["sh", "-c", "java $JAVA_OPTS -jar app.jar"]
