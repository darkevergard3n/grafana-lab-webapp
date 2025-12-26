/**
 * =============================================================================
 * USER REPOSITORY
 * =============================================================================
 * Spring Data JPA Repository for User entity.
 * Spring automatically implements these methods based on method names.
 * =============================================================================
 */
package com.example.userservice.repository;

import com.example.userservice.model.User;
import org.springframework.data.domain.Page;
import org.springframework.data.domain.Pageable;
import org.springframework.data.jpa.repository.JpaRepository;
import org.springframework.stereotype.Repository;

import java.util.Optional;
import java.util.UUID;

@Repository
public interface UserRepository extends JpaRepository<User, UUID> {
    
    // Spring Data JPA generates query: SELECT * FROM users WHERE email = ?
    Optional<User> findByEmail(String email);
    
    // Check if email exists
    boolean existsByEmail(String email);
    
    // Find active users with pagination
    Page<User> findByIsActiveTrue(Pageable pageable);
}
