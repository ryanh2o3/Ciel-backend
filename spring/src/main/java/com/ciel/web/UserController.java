package com.ciel.web;

import com.ciel.app.AuthService;
import com.ciel.app.MediaService;
import com.ciel.app.UserService;
import com.ciel.domain.PublicUser;
import com.ciel.domain.User;
import com.ciel.web.dto.CreateUserRequest;
import com.ciel.web.error.ApiException;
import org.springframework.dao.DataIntegrityViolationException;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.PathVariable;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RestController;

import java.util.UUID;
import java.util.regex.Pattern;

@RestController
@RequestMapping("/v1/users")
public class UserController {

    private static final Pattern HANDLE = Pattern.compile("^[a-zA-Z0-9_]{3,30}$");

    private final AuthService authService;
    private final UserService userService;
    private final MediaService mediaService;

    public UserController(AuthService authService, UserService userService, MediaService mediaService) {
        this.authService = authService;
        this.userService = userService;
        this.mediaService = mediaService;
    }

    @PostMapping
    public User createUser(@RequestBody CreateUserRequest body) {
        validateSignup(body);
        try {
            User user = authService.signup(
                    body.getHandle().trim(),
                    body.getEmail().trim(),
                    body.getDisplayName().trim(),
                    body.getBio(),
                    body.getAvatarKey(),
                    body.getPassword(),
                    body.getInviteCode().trim());
            mediaService.populateUserAvatarUrl(user);
            return user;
        } catch (DataIntegrityViolationException e) {
            String msg = e.getMostSpecificCause().getMessage();
            if (msg != null && msg.contains("users_handle_key")) {
                throw ApiException.conflict("Handle already taken");
            }
            if (msg != null && msg.contains("users_email_key")) {
                throw ApiException.conflict("Email already registered");
            }
            throw ApiException.badRequest("invalid signup request");
        }
    }

    @GetMapping("/{id}")
    public PublicUser getUser(@PathVariable UUID id) {
        UserService.PublicUserWithCounts result = userService
                .getPublicUserWithCounts(id)
                .orElseThrow(() -> ApiException.notFound("user not found"));
        mediaService.populateUserAvatarUrl(result.user());
        PublicUser pub = PublicUser.fromUser(result.user());
        pub.setFollowersCount(result.followersCount());
        pub.setFollowingCount(result.followingCount());
        pub.setPostsCount(result.postsCount());
        return pub;
    }

    private static void validateSignup(CreateUserRequest body) {
        if (body.getHandle() == null || !HANDLE.matcher(body.getHandle().trim()).matches()) {
            throw ApiException.badRequest("handle must be 3-30 alphanumeric characters or underscores");
        }
        if (body.getEmail() == null || body.getEmail().trim().isEmpty()) {
            throw ApiException.badRequest("email cannot be empty");
        }
        String email = body.getEmail().trim();
        if (email.length() > 254) {
            throw ApiException.badRequest("email must be at most 254 characters");
        }
        String[] parts = email.split("@");
        if (parts.length != 2 || parts[0].isEmpty() || !parts[1].contains(".")) {
            throw ApiException.badRequest("invalid email format");
        }
        if (body.getDisplayName() == null || body.getDisplayName().trim().isEmpty()) {
            throw ApiException.badRequest("display_name cannot be empty");
        }
        if (body.getDisplayName().length() > 50) {
            throw ApiException.badRequest("display_name must be at most 50 characters");
        }
        if (body.getBio() != null && body.getBio().length() > 500) {
            throw ApiException.badRequest("bio must be at most 500 characters");
        }
        if (body.getPassword() == null || body.getPassword().trim().length() < 8) {
            throw ApiException.badRequest("password must be at least 8 characters");
        }
        if (body.getPassword().length() > 128) {
            throw ApiException.badRequest("password must be at most 128 characters");
        }
        if (body.getPassword().chars().noneMatch(Character::isUpperCase)) {
            throw ApiException.badRequest("password must contain at least one uppercase letter");
        }
        if (body.getPassword().chars().noneMatch(Character::isLowerCase)) {
            throw ApiException.badRequest("password must contain at least one lowercase letter");
        }
        if (body.getPassword().chars().noneMatch(Character::isDigit)) {
            throw ApiException.badRequest("password must contain at least one digit");
        }
        if (body.getInviteCode() == null || body.getInviteCode().trim().isEmpty()) {
            throw ApiException.badRequest("invite_code is required");
        }
    }
}
