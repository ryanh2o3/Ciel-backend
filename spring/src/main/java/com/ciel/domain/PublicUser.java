package com.ciel.domain;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

import java.time.OffsetDateTime;
import java.util.UUID;

/** Mirrors Rust's {@code domain::user::PublicUser}. */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class PublicUser {
    private UUID id;
    private String handle;
    private String displayName;
    private String bio;
    private String avatarUrl;
    private OffsetDateTime createdAt;
    private long followersCount;
    private long followingCount;
    private long postsCount;

    public static PublicUser fromUser(User user) {
        return PublicUser.builder()
                .id(user.getId())
                .handle(user.getHandle())
                .displayName(user.getDisplayName())
                .bio(user.getBio())
                .avatarUrl(user.getAvatarUrl())
                .createdAt(user.getCreatedAt())
                .followersCount(0)
                .followingCount(0)
                .postsCount(0)
                .build();
    }
}
