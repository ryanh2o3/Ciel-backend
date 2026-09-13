package com.ciel.domain;

import com.fasterxml.jackson.annotation.JsonIgnore;
import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

import java.time.OffsetDateTime;
import java.util.UUID;

/** Mirrors Rust's {@code domain::user::User}. */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class User {
    private UUID id;
    private String handle;
    private String email;
    private String displayName;
    private String bio;

    /** Raw S3 key — never serialized (matches Rust's {@code #[serde(skip_serializing)]}). */
    @JsonIgnore
    private String avatarKey;

    /** Presigned URL for the avatar, populated at response time. */
    private String avatarUrl;

    private OffsetDateTime createdAt;
}
