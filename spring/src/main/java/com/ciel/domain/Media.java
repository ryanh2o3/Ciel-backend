package com.ciel.domain;

import com.fasterxml.jackson.annotation.JsonInclude;
import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

import java.time.OffsetDateTime;
import java.util.UUID;

/** Mirrors Rust's {@code domain::media::Media}. */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
@JsonInclude(JsonInclude.Include.ALWAYS)
public class Media {
    private UUID id;
    private UUID ownerId;
    private String originalKey;
    private String thumbKey;
    private String mediumKey;
    private int width;
    private int height;
    private long bytes;
    private OffsetDateTime createdAt;
    private String thumbUrl;
    private String mediumUrl;
    private String originalUrl;
}
