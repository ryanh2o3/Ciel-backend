package com.ciel.domain;

import com.fasterxml.jackson.annotation.JsonInclude;
import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

import java.time.OffsetDateTime;
import java.util.UUID;

/** Mirrors Rust's {@code domain::engagement::Comment}. */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class Comment {
    private UUID id;
    private UUID userId;
    private UUID postId;
    private String body;
    private OffsetDateTime createdAt;

    @JsonInclude(JsonInclude.Include.NON_NULL)
    private String userHandle;

    @JsonInclude(JsonInclude.Include.NON_NULL)
    private String userDisplayName;
}
