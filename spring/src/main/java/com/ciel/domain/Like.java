package com.ciel.domain;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

import java.time.OffsetDateTime;
import java.util.UUID;

/** Mirrors Rust's {@code domain::engagement::Like}. */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class Like {
    private UUID id;
    private UUID userId;
    private UUID postId;
    private OffsetDateTime createdAt;
}
