/**
 * =============================================================================
 * USER CONTROLLER
 * =============================================================================
 * Handles HTTP requests for user-related operations.
 * 
 * REST ENDPOINTS:
 * - POST /api/v1/users/register - Register new user
 * - POST /api/v1/users/login    - Authenticate user
 * - GET  /api/v1/users/me       - Get current user
 * - GET  /api/v1/users/:id      - Get user by ID
 * - PUT  /api/v1/users/:id      - Update user
 * - GET  /api/v1/users          - List all users (admin)
 * =============================================================================
 */
package com.example.userservice.controller;

import com.example.userservice.model.User;
import com.example.userservice.model.LoginRequest;
import com.example.userservice.model.LoginResponse;
import com.example.userservice.model.RegisterRequest;
import com.example.userservice.service.UserService;

import io.micrometer.core.instrument.Counter;
import io.micrometer.core.instrument.MeterRegistry;

import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.http.HttpStatus;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.*;

import jakarta.validation.Valid;
import java.util.List;
import java.util.Map;
import java.util.UUID;

/**
 * REST Controller for user operations.
 * 
 * @RestController combines @Controller and @ResponseBody
 * @RequestMapping sets the base path for all endpoints
 */
@RestController
@RequestMapping("/api/v1/users")
public class UserController {

    private final UserService userService;
    private final Counter usersRegisteredCounter;
    private final Counter loginAttemptsCounter;

    /**
     * Constructor injection (preferred over field injection)
     * Spring automatically injects dependencies.
     */
    @Autowired
    public UserController(UserService userService, MeterRegistry registry) {
        this.userService = userService;
        
        // Create Prometheus metrics
        this.usersRegisteredCounter = Counter.builder("users_registered_total")
            .description("Total number of users registered")
            .register(registry);
            
        this.loginAttemptsCounter = Counter.builder("login_attempts_total")
            .description("Total number of login attempts")
            .tag("status", "success") // Will be overridden per call
            .register(registry);
    }

    /**
     * Register a new user.
     * 
     * POST /api/v1/users/register
     * 
     * @param request Registration details
     * @return Created user (without password)
     */
    @PostMapping("/register")
    public ResponseEntity<?> register(@Valid @RequestBody RegisterRequest request) {
        try {
            User user = userService.register(request);
            usersRegisteredCounter.increment();
            return ResponseEntity.status(HttpStatus.CREATED).body(user);
        } catch (IllegalArgumentException e) {
            return ResponseEntity.badRequest()
                .body(Map.of("error", e.getMessage()));
        }
    }

    /**
     * Authenticate user and return JWT token.
     * 
     * POST /api/v1/users/login
     * 
     * @param request Login credentials
     * @return JWT token if successful
     */
    @PostMapping("/login")
    public ResponseEntity<?> login(@Valid @RequestBody LoginRequest request) {
        try {
            LoginResponse response = userService.login(request);
            return ResponseEntity.ok(response);
        } catch (Exception e) {
            return ResponseEntity.status(HttpStatus.UNAUTHORIZED)
                .body(Map.of("error", "Invalid credentials"));
        }
    }

    /**
     * Get current authenticated user.
     * 
     * GET /api/v1/users/me
     * 
     * @param authHeader Authorization header with JWT
     * @return Current user details
     */
    @GetMapping("/me")
    public ResponseEntity<?> getCurrentUser(
            @RequestHeader(value = "Authorization", required = false) String authHeader) {
        if (authHeader == null || !authHeader.startsWith("Bearer ")) {
            return ResponseEntity.status(HttpStatus.UNAUTHORIZED)
                .body(Map.of("error", "Missing or invalid authorization header"));
        }
        
        try {
            String token = authHeader.substring(7);
            User user = userService.getUserFromToken(token);
            return ResponseEntity.ok(user);
        } catch (Exception e) {
            return ResponseEntity.status(HttpStatus.UNAUTHORIZED)
                .body(Map.of("error", "Invalid token"));
        }
    }

    /**
     * Get user by ID.
     * 
     * GET /api/v1/users/:id
     * 
     * @param id User UUID
     * @return User details
     */
    @GetMapping("/{id}")
    public ResponseEntity<?> getUserById(@PathVariable UUID id) {
        return userService.findById(id)
            .map(ResponseEntity::ok)
            .orElse(ResponseEntity.notFound().build());
    }

    /**
     * Update user profile.
     * 
     * PUT /api/v1/users/:id
     * 
     * @param id User UUID
     * @param updates Fields to update
     * @return Updated user
     */
    @PutMapping("/{id}")
    public ResponseEntity<?> updateUser(
            @PathVariable UUID id,
            @RequestBody Map<String, Object> updates) {
        try {
            User user = userService.update(id, updates);
            return ResponseEntity.ok(user);
        } catch (Exception e) {
            return ResponseEntity.badRequest()
                .body(Map.of("error", e.getMessage()));
        }
    }

    /**
     * List all users (admin only).
     * 
     * GET /api/v1/users
     * 
     * @param page Page number (0-indexed)
     * @param size Page size
     * @return Paginated list of users
     */
    @GetMapping
    public ResponseEntity<List<User>> listUsers(
            @RequestParam(defaultValue = "0") int page,
            @RequestParam(defaultValue = "20") int size) {
        List<User> users = userService.findAll(page, size);
        return ResponseEntity.ok(users);
    }
}
