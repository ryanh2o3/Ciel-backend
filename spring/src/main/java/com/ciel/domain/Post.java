package com.ciel.domain;

import com.fasterxml.jackson.annotation.JsonInclude;
import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

import java.time.OffsetDateTime;
import java.util.List;
import java.util.UUID;

/** Mirrors Rust's {@code domain::post::Post}. */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
@JsonInclude(JsonInclude.Include.ALWAYS)
public class Post {
    private UUID id;
    private UUID ownerId;
    private String ownerHandle;
    private String ownerDisplayName;
    private List<UUID> mediaIds;

    @JsonInclude(JsonInclude.Include.NON_NULL)
    private Media primaryMedia;

    private String caption;
    private OffsetDateTime createdAt;
    private PostVisibility visibility;

    @JsonInclude(JsonInclude.Include.NON_NULL)
    private String ownerAvatarKey;

    @JsonInclude(JsonInclude.Include.NON_NULL)
    private String ownerAvatarUrl;

    @JsonInclude(JsonInclude.Include.NON_NULL)
    private Long likeCount;

    @JsonInclude(JsonInclude.Include.NON_NULL)
    private Long commentCount;

    @JsonInclude(JsonInclude.Include.NON_NULL)
    private Boolean likedByViewer;
}
