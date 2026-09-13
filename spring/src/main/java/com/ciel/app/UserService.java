package com.ciel.app;

import com.ciel.domain.User;
import org.springframework.jdbc.core.JdbcTemplate;
import org.springframework.stereotype.Service;

import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Timestamp;
import java.time.OffsetDateTime;
import java.time.ZoneOffset;
import java.util.Optional;
import java.util.UUID;

@Service
public class UserService {

    private final JdbcTemplate jdbc;

    public UserService(JdbcTemplate jdbc) {
        this.jdbc = jdbc;
    }

    public record PublicUserWithCounts(User user, long followersCount, long followingCount, long postsCount) {}

    public Optional<PublicUserWithCounts> getPublicUserWithCounts(UUID userId) {
        return jdbc.query(
                """
                SELECT u.id, u.handle, u.email, u.display_name, u.bio, u.avatar_key, u.created_at,
                       (SELECT COUNT(*) FROM follows WHERE followee_id = u.id) AS followers_count,
                       (SELECT COUNT(*) FROM follows WHERE follower_id = u.id) AS following_count,
                       (SELECT COUNT(*) FROM posts WHERE owner_id = u.id) AS posts_count
                FROM users u WHERE u.id = ? AND u.deleted_at IS NULL
                """,
                rs -> {
                    if (!rs.next()) {
                        return Optional.<PublicUserWithCounts>empty();
                    }
                    User user = mapUser(rs);
                    long followers = rs.getLong("followers_count");
                    long following = rs.getLong("following_count");
                    long posts = rs.getLong("posts_count");
                    return Optional.of(new PublicUserWithCounts(user, followers, following, posts));
                },
                userId);
    }

    static User mapUser(ResultSet rs) throws SQLException {
        Timestamp created = rs.getTimestamp("created_at");
        return User.builder()
                .id((UUID) rs.getObject("id"))
                .handle(rs.getString("handle"))
                .email(rs.getString("email"))
                .displayName(rs.getString("display_name"))
                .bio(rs.getString("bio"))
                .avatarKey(rs.getString("avatar_key"))
                .createdAt(created == null ? null : OffsetDateTime.ofInstant(created.toInstant(), ZoneOffset.UTC))
                .build();
    }
}
