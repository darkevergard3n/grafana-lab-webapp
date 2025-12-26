/**
 * =============================================================================
 * USER SERVICE
 * =============================================================================
 * Business logic for user operations.
 * =============================================================================
 */
package com.example.userservice.service;

import com.example.userservice.model.*;
import com.example.userservice.repository.UserRepository;

import io.jsonwebtoken.Jwts;
import io.jsonwebtoken.security.Keys;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.data.domain.PageRequest;
import org.springframework.security.crypto.bcrypt.BCryptPasswordEncoder;
import org.springframework.stereotype.Service;

import javax.crypto.SecretKey;
import java.nio.charset.StandardCharsets;
import java.time.Instant;
import java.util.*;

@Service
public class UserService {

    private static final Logger logger = LoggerFactory.getLogger(UserService.class);

    private final UserRepository userRepository;
    private final BCryptPasswordEncoder passwordEncoder;
    private final SecretKey jwtKey;
    private final long jwtExpiration;

    @Autowired
    public UserService(
            UserRepository userRepository,
            @Value("${jwt.secret}") String jwtSecret,
            @Value("${jwt.expiration}") long jwtExpiration) {
        this.userRepository = userRepository;
        this.passwordEncoder = new BCryptPasswordEncoder();
        this.jwtKey = Keys.hmacShaKeyFor(jwtSecret.getBytes(StandardCharsets.UTF_8));
        this.jwtExpiration = jwtExpiration;
    }

    public User register(RegisterRequest request) {
        logger.info("Registering new user: email={}", request.getEmail());
        
        if (userRepository.existsByEmail(request.getEmail())) {
            logger.warn("Registration failed - email already exists: {}", request.getEmail());
            throw new IllegalArgumentException("Email already registered");
        }

        User user = new User();
        user.setEmail(request.getEmail());
        user.setPasswordHash(passwordEncoder.encode(request.getPassword()));
        user.setName(request.getName());

        User savedUser = userRepository.save(user);
        logger.info("User registered successfully: id={}, email={}", savedUser.getId(), savedUser.getEmail());
        
        return savedUser;
    }

    public LoginResponse login(LoginRequest request) {
        logger.info("Login attempt: email={}", request.getEmail());
        
        User user = userRepository.findByEmail(request.getEmail())
                .orElseThrow(() -> {
                    logger.warn("Login failed - user not found: {}", request.getEmail());
                    return new RuntimeException("Invalid credentials");
                });

        if (!passwordEncoder.matches(request.getPassword(), user.getPasswordHash())) {
            logger.warn("Login failed - invalid password: email={}", request.getEmail());
            throw new RuntimeException("Invalid credentials");
        }

        user.setLastLoginAt(Instant.now());
        userRepository.save(user);

        String token = generateToken(user);
        
        logger.info("Login successful: id={}, email={}", user.getId(), user.getEmail());
        
        return new LoginResponse(token, user);
    }

    public User getUserFromToken(String token) {
        String userId = Jwts.parser()
                .verifyWith(jwtKey)
                .build()
                .parseSignedClaims(token)
                .getPayload()
                .getSubject();

        logger.info("Token validated for user: id={}", userId);
        
        return userRepository.findById(UUID.fromString(userId))
                .orElseThrow(() -> new RuntimeException("User not found"));
    }

    public Optional<User> findById(UUID id) {
        logger.info("Fetching user by id: {}", id);
        return userRepository.findById(id);
    }

    public List<User> findAll(int page, int size) {
        logger.info("Listing users: page={}, size={}", page, size);
        return userRepository.findAll(PageRequest.of(page, size)).getContent();
    }

    public User update(UUID id, Map<String, Object> updates) {
        logger.info("Updating user: id={}, fields={}", id, updates.keySet());
        
        User user = userRepository.findById(id)
                .orElseThrow(() -> {
                    logger.warn("Update failed - user not found: {}", id);
                    return new RuntimeException("User not found");
                });

        if (updates.containsKey("name")) {
            user.setName((String) updates.get("name"));
        }

        User savedUser = userRepository.save(user);
        logger.info("User updated successfully: id={}", id);
        
        return savedUser;
    }

    private String generateToken(User user) {
        return Jwts.builder()
                .subject(user.getId().toString())
                .claim("email", user.getEmail())
                .claim("role", user.getRole())
                .issuedAt(new Date())
                .expiration(new Date(System.currentTimeMillis() + jwtExpiration))
                .signWith(jwtKey)
                .compact();
    }
}
