package com.ciel.web;

import com.ciel.app.AuthService;
import com.ciel.app.MediaService;
import com.ciel.app.UserService;
import com.ciel.domain.PublicUser;
import com.ciel.domain.User;
import com.ciel.web.dto.CreateUserRequest;
import com.ciel.web.error.ApiException;
import org.postgresql.util.PSQLException;
import org.springframework.dao.DataIntegrityViolationException;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.PathVariable;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RestController;

import java.sql.SQLException;
import java.util.UUID;

import static com.ciel.web.RequestValidation.MAX_BIO_LEN;
import static com.ciel.web.RequestValidation.MAX_DISPLAY_NAME_LEN;
import static com.ciel.web.RequestValidation.MAX_EMAIL_LEN;
import static com.ciel.web.RequestValidation.MAX_PASSWORD_LEN;

@RestController
@RequestMapping("/v1/users")
public class UserController {

    private static final String UNIQUE_VIOLATION_SQLSTATE = "23505";

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
            // Spring JDBC always wraps SQLExceptions in a DataAccessException
            // subtype; the underlying driver exception (PSQLException) is in
            // the cause chain, which is where the SQLState / constraint name
            // actually live.
            throw mapUniqueViolation(e);
        }
    }

    /**
     * Maps a unique-constraint violation (SQLState {@code 23505}) to the same
     * per-field conflict messages Rust returns in {@code handlers.rs}, using the
     * constraint name when the driver exposes it and falling back to a generic
     * conflict otherwise. Any other database error is re-thrown as-is so it
     * reaches {@link com.ciel.web.error.ApiExceptionHandler}'s generic 500 path.
     */
    private static ApiException mapUniqueViolation(RuntimeException ex) {
        PSQLException psql = findPsqlException(ex);
        if (psql == null || !UNIQUE_VIOLATION_SQLSTATE.equals(psql.getSQLState())) {
            throw ex;
        }
        String constraint = psql.getServerErrorMessage() != null
                ? psql.getServerErrorMessage().getConstraint()
                : null;
        if (constraint != null && constraint.contains("users_handle_key")) {
            return ApiException.conflict("Handle already taken");
        }
        if (constraint != null && constraint.contains("users_email_key")) {
            return ApiException.conflict("Email already taken");
        }
        // Constraint name unavailable or unrecognized (driver-dependent): fall
        // back to a generic conflict rather than guessing at the field.
        return ApiException.conflict("user already exists");
    }

    private static PSQLException findPsqlException(Throwable ex) {
        Throwable cause = ex;
        while (cause != null) {
            if (cause instanceof PSQLException psql) {
                return psql;
            }
            if (cause instanceof SQLException sql && sql.getCause() instanceof PSQLException psql) {
                return psql;
            }
            cause = cause.getCause();
        }
        return null;
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
        RequestValidation.validateHandle(body.getHandle());

        if (body.getEmail() == null || body.getEmail().trim().isEmpty()) {
            throw ApiException.badRequest("email cannot be empty");
        }
        String email = body.getEmail().trim();
        RequestValidation.validateMaxLen("email", email, MAX_EMAIL_LEN);
        String[] parts = email.split("@");
        if (parts.length != 2 || parts[0].isEmpty() || !parts[1].contains(".")) {
            throw ApiException.badRequest("invalid email format");
        }

        if (body.getDisplayName() == null || body.getDisplayName().trim().isEmpty()) {
            throw ApiException.badRequest("display_name cannot be empty");
        }
        RequestValidation.validateMaxLen("display_name", body.getDisplayName(), MAX_DISPLAY_NAME_LEN);

        if (body.getBio() != null) {
            RequestValidation.validateMaxLen("bio", body.getBio(), MAX_BIO_LEN);
        }

        if (body.getPassword() == null || body.getPassword().trim().length() < 8) {
            throw ApiException.badRequest("password must be at least 8 characters");
        }
        RequestValidation.validateMaxLen("password", body.getPassword(), MAX_PASSWORD_LEN);
        if (body.getPassword().chars().noneMatch(Character::isUpperCase)) {
            throw ApiException.badRequest("password must contain at least one uppercase letter");
        }
        if (body.getPassword().chars().noneMatch(Character::isLowerCase)) {
            throw ApiException.badRequest("password must contain at least one lowercase letter");
        }
        if (body.getPassword().chars().noneMatch(Character::isDigit)) {
            throw ApiException.badRequest("password must contain at least one digit");
        }
        RequestValidation.requiredTrimmed("invite_code", body.getInviteCode());
    }
}
