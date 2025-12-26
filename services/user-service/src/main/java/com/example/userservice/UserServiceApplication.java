/**
 * =============================================================================
 * USER SERVICE - Main Application (Java + Spring Boot)
 * =============================================================================
 * This is the main entry point for the User Service.
 *
 * WHAT THIS SERVICE DOES:
 * - User registration and authentication
 * - JWT token generation and validation
 * - User profile management
 * - Role-based access control (RBAC)
 *
 * WHY JAVA/SPRING?
 * - Enterprise standard for authentication systems
 * - Mature security frameworks (Spring Security)
 * - Excellent Prometheus integration (Micrometer)
 * - Battle-tested in production environments
 *
 * LEARNING GOALS:
 * - Understand Spring Boot auto-configuration
 * - Learn Spring Data JPA for database access
 * - See Micrometer metrics integration
 * - Understand JWT authentication flow
 * =============================================================================
 */
package com.example.userservice;

import org.springframework.boot.SpringApplication;
import org.springframework.boot.autoconfigure.SpringBootApplication;

/**
 * Main application class for the User Service.
 * 
 * The @SpringBootApplication annotation is a convenience annotation that:
 * - @Configuration: Tags the class as a source of bean definitions
 * - @EnableAutoConfiguration: Tells Spring Boot to configure based on classpath
 * - @ComponentScan: Tells Spring to scan for components in this package
 */
@SpringBootApplication
public class UserServiceApplication {

    /**
     * Main entry point for the application.
     * 
     * @param args Command line arguments
     */
    public static void main(String[] args) {
        // SpringApplication.run() bootstraps the application:
        // 1. Creates Spring ApplicationContext
        // 2. Registers beans
        // 3. Starts embedded web server
        SpringApplication.run(UserServiceApplication.class, args);
    }
}
