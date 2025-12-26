-- =============================================================================
-- POSTGRESQL INITIALIZATION SCRIPT
-- =============================================================================
-- This script runs when PostgreSQL container starts for the first time.
-- It creates the database schema and initial data.
--
-- LEARNING NOTES:
-- - Scripts in docker-entrypoint-initdb.d/ run alphabetically
-- - Only runs on fresh database (empty data directory)
-- - Use IF NOT EXISTS for idempotency
-- =============================================================================

-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- =============================================================================
-- USERS TABLE (for user-service)
-- =============================================================================
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    name VARCHAR(255) NOT NULL,
    role VARCHAR(50) NOT NULL DEFAULT 'user',
    is_active BOOLEAN NOT NULL DEFAULT true,
    email_verified BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_login_at TIMESTAMPTZ
);

-- Index for email lookups (login)
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);

-- Index for role-based queries
CREATE INDEX IF NOT EXISTS idx_users_role ON users(role);

-- =============================================================================
-- SAMPLE DATA
-- =============================================================================
-- Insert sample users (password is 'password123' hashed with bcrypt)
INSERT INTO users (email, password_hash, name, role, email_verified)
VALUES 
    ('admin@example.com', '$2a$10$rPfV7xhDWx0UD9qFrGJpOeq5TYr5hIwVMSMjC2xBzQhKk5U6cJvJe', 'System Admin', 'admin', true),
    ('user@example.com', '$2a$10$rPfV7xhDWx0UD9qFrGJpOeq5TYr5hIwVMSMjC2xBzQhKk5U6cJvJe', 'Demo User', 'user', true)
ON CONFLICT (email) DO NOTHING;

-- =============================================================================
-- GRANT PERMISSIONS
-- =============================================================================
-- Grant all privileges to the webapp user
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO webapp;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO webapp;
