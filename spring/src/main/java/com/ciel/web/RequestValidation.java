package com.ciel.web;

import com.ciel.web.error.ApiException;

/**
 * Shared request-validation helpers. Mirrors Rust's {@code http::validation}
 * module — constants and error message strings are kept byte-for-byte
 * identical so both implementations return the same {@code {"error": "..."}}
 * body for the same invalid input.
 */
public final class RequestValidation {

    public static final int MAX_HANDLE_LEN = 30;
    public static final int MIN_HANDLE_LEN = 3;
    public static final int MAX_DISPLAY_NAME_LEN = 50;
    public static final int MAX_BIO_LEN = 500;
    public static final int MAX_CAPTION_LEN = 2200;
    public static final int MAX_PASSWORD_LEN = 128;
    public static final int MAX_EMAIL_LEN = 254;
    public static final int MAX_SEARCH_QUERY_LEN = 100;
    public static final int MAX_HIGHLIGHT_NAME_LEN = 50;
    public static final int MAX_FINGERPRINT_LEN = 512;
    public static final int MAX_COMMENT_LEN = 1000;

    private RequestValidation() {}

    /** Trims {@code value} and rejects it if empty, matching Rust's {@code required_trimmed}. */
    public static String requiredTrimmed(String field, String value) {
        String trimmed = value == null ? "" : value.trim();
        if (trimmed.isEmpty()) {
            throw ApiException.badRequest(field + " is required");
        }
        return trimmed;
    }

    /** Matches Rust's {@code validate_max_len}. */
    public static void validateMaxLen(String field, String value, int maxLen) {
        if (value != null && value.length() > maxLen) {
            throw ApiException.badRequest(field + " must be at most " + maxLen + " characters");
        }
    }

    /** Matches Rust's {@code validate_handle} exactly, including error message strings. */
    public static void validateHandle(String handle) {
        String trimmed = handle == null ? "" : handle.trim();
        if (trimmed.length() < MIN_HANDLE_LEN) {
            throw ApiException.badRequest("handle must be at least 3 characters");
        }
        if (trimmed.length() > MAX_HANDLE_LEN) {
            throw ApiException.badRequest("handle must be at most 30 characters");
        }
        for (int i = 0; i < trimmed.length(); i++) {
            char c = trimmed.charAt(i);
            boolean asciiAlphanumeric = (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9');
            if (!asciiAlphanumeric && c != '_') {
                throw ApiException.badRequest("handle can only contain letters, numbers, and underscores");
            }
        }
    }
}
