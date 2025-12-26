package com.example.userservice.model;

public class LoginResponse {
    private String token;
    private String type = "Bearer";
    private User user;

    public LoginResponse(String token, User user) {
        this.token = token;
        this.user = user;
    }

    public String getToken() { return token; }
    public String getType() { return type; }
    public User getUser() { return user; }
}
