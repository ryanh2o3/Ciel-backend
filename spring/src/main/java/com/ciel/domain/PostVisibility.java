package com.ciel.domain;

import com.fasterxml.jackson.annotation.JsonCreator;
import com.fasterxml.jackson.annotation.JsonValue;

/**
 * Mirrors Rust's {@code domain::post::PostVisibility}. JSON representation
 * follows the Rust serde enum names verbatim ({@code Public}, {@code FollowersOnly});
 * the DB representation is lowercase snake_case ({@code public}, {@code followers_only}).
 */
public enum PostVisibility {
    PUBLIC("public", "Public"),
    FOLLOWERS_ONLY("followers_only", "FollowersOnly");

    private final String db;
    private final String json;

    PostVisibility(String db, String json) {
        this.db = db;
        this.json = json;
    }

    @JsonValue
    public String jsonName() {
        return json;
    }

    public String asDb() {
        return db;
    }

    public static PostVisibility fromDb(String value) {
        if (value == null) {
            return null;
        }
        return switch (value) {
            case "public" -> PUBLIC;
            case "followers_only" -> FOLLOWERS_ONLY;
            default -> null;
        };
    }

    @JsonCreator
    public static PostVisibility fromJson(String value) {
        for (PostVisibility v : values()) {
            if (v.json.equals(value)) {
                return v;
            }
        }
        throw new IllegalArgumentException("unknown post visibility: " + value);
    }
}
