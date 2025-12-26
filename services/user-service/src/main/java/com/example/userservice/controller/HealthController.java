/**
 * =============================================================================
 * HEALTH CONTROLLER
 * =============================================================================
 * Health check endpoints for Kubernetes/Docker orchestration.
 * =============================================================================
 */
package com.example.userservice.controller;

import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.RestController;

import javax.sql.DataSource;
import java.sql.Connection;
import java.util.HashMap;
import java.util.Map;

@RestController
public class HealthController {

    @Autowired
    private DataSource dataSource;

    @GetMapping("/health")
    public ResponseEntity<Map<String, Object>> health() {
        Map<String, Object> response = new HashMap<>();
        response.put("status", "ok");
        response.put("service", "user-service");
        response.put("version", "1.0.0");
        return ResponseEntity.ok(response);
    }

    @GetMapping("/ready")
    public ResponseEntity<Map<String, Object>> ready() {
        Map<String, Object> response = new HashMap<>();
        Map<String, Boolean> checks = new HashMap<>();

        // Check database
        boolean dbHealthy = false;
        try (Connection conn = dataSource.getConnection()) {
            dbHealthy = conn.isValid(5);
        } catch (Exception e) {
            // Database not healthy
        }
        checks.put("database", dbHealthy);

        boolean allHealthy = dbHealthy;
        response.put("status", allHealthy ? "ready" : "not_ready");
        response.put("checks", checks);

        if (allHealthy) {
            return ResponseEntity.ok(response);
        } else {
            return ResponseEntity.status(503).body(response);
        }
    }
}
