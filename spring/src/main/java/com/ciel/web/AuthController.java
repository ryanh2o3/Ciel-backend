package com.ciel.web;

import com.ciel.app.AuthService;
import com.ciel.app.MediaService;
import com.ciel.domain.User;
import com.ciel.web.auth.AuthUser;
import com.ciel.web.dto.AuthTokenResponse;
import com.ciel.web.dto.LoginRequest;
import com.ciel.web.dto.RefreshRequest;
import com.ciel.web.dto.RevokeRequest;
import com.ciel.web.error.ApiException;
import org.springframework.http.HttpStatus;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.ResponseStatus;
import org.springframework.web.bind.annotation.RestController;

@RestController
@RequestMapping("/v1/auth")
public class AuthController {

    private final AuthService authService;
    private final MediaService mediaService;

    public AuthController(AuthService authService, MediaService mediaService) {
        this.authService = authService;
        this.mediaService = mediaService;
    }

    @PostMapping("/login")
    public AuthTokenResponse login(@RequestBody LoginRequest body) {
        if (body.getEmail() == null
                || body.getEmail().trim().isEmpty()
                || body.getPassword() == null
                || body.getPassword().trim().isEmpty()) {
            throw ApiException.badRequest("email and password are required");
        }
        if (body.getPassword().length() > 128) {
            throw ApiException.badRequest("password must be at most 128 characters");
        }
        return authService.login(body.getEmail().trim(), body.getPassword());
    }

    @PostMapping("/refresh")
    public AuthTokenResponse refresh(@RequestBody RefreshRequest body) {
        if (body.getRefreshToken() == null || body.getRefreshToken().trim().isEmpty()) {
            throw ApiException.badRequest("refresh_token is required");
        }
        return authService.refresh(body.getRefreshToken().trim());
    }

    @PostMapping("/revoke")
    @ResponseStatus(HttpStatus.NO_CONTENT)
    public void revoke(@RequestBody RevokeRequest body) {
        if (body.getRefreshToken() == null || body.getRefreshToken().trim().isEmpty()) {
            throw ApiException.badRequest("refresh_token is required");
        }
        authService.revoke(body.getRefreshToken().trim());
    }

    @GetMapping("/me")
    public User me(AuthUser auth) {
        User user = authService
                .getCurrentUser(auth.userId())
                .orElseThrow(() -> ApiException.notFound("user not found"));
        mediaService.populateUserAvatarUrl(user);
        return user;
    }
}
