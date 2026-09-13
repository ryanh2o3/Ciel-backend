package com.ciel.app;

import com.ciel.domain.Post;
import com.ciel.domain.PostVisibility;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.PropertyNamingStrategies;
import com.fasterxml.jackson.datatype.jsr310.JavaTimeModule;
import org.junit.jupiter.api.Test;

import java.time.OffsetDateTime;
import java.util.List;
import java.util.Optional;
import java.util.UUID;

import static org.assertj.core.api.Assertions.assertThat;

/**
 * {@code FeedService.CachedHomeFeed} is read/written by both the Rust and
 * Spring implementations against the same {@code feed:home:{user_id}} Redis
 * key, so its JSON field names must match Rust's serde field names exactly
 * ({@code posts}, {@code next_cursor_nanos}, {@code next_cursor_id}).
 */
class FeedServiceCachedHomeFeedTest {

    private static ObjectMapper mapper() {
        ObjectMapper mapper = new ObjectMapper();
        mapper.registerModule(new JavaTimeModule());
        mapper.setPropertyNamingStrategy(PropertyNamingStrategies.SNAKE_CASE);
        return mapper;
    }

    private static Post samplePost(UUID id) {
        return Post.builder()
                .id(id)
                .ownerId(UUID.randomUUID())
                .ownerHandle("alice")
                .ownerDisplayName("Alice")
                .mediaIds(List.of(UUID.randomUUID()))
                .visibility(PostVisibility.PUBLIC)
                .createdAt(OffsetDateTime.now())
                .build();
    }

    @Test
    void serializesExpectedFieldNames() throws Exception {
        ObjectMapper mapper = mapper();
        UUID cursorId = UUID.randomUUID();
        OffsetDateTime cursorTs = OffsetDateTime.now();

        FeedService.CachedHomeFeed cached = FeedService.CachedHomeFeed.fromPage(
                List.of(samplePost(UUID.randomUUID())),
                Optional.of(new CursorUtil.Cursor(cursorTs, cursorId)));

        JsonNode node = mapper.readTree(mapper.writeValueAsString(cached));

        assertThat(node.has("posts")).isTrue();
        assertThat(node.has("next_cursor_nanos")).isTrue();
        assertThat(node.has("next_cursor_id")).isTrue();
        assertThat(node.get("next_cursor_id").asText()).isEqualTo(cursorId.toString());
    }

    @Test
    void roundTripsThroughPageAndBack() throws Exception {
        ObjectMapper mapper = mapper();
        UUID postId = UUID.randomUUID();
        UUID cursorId = UUID.randomUUID();
        OffsetDateTime cursorTs = OffsetDateTime.now();

        FeedService.CachedHomeFeed cached = FeedService.CachedHomeFeed.fromPage(
                List.of(samplePost(postId)),
                Optional.of(new CursorUtil.Cursor(cursorTs, cursorId)));

        String json = mapper.writeValueAsString(cached);
        FeedService.CachedHomeFeed deserialized = mapper.readValue(json, FeedService.CachedHomeFeed.class);
        FeedService.FeedPage page = deserialized.toPage();

        assertThat(page).isNotNull();
        assertThat(page.posts()).extracting(Post::getId).containsExactly(postId);
        assertThat(page.nextCursor()).isPresent();
        assertThat(page.nextCursor().get().id()).isEqualTo(cursorId);
    }

    @Test
    void noCursorOmitsCursorFieldsAndRoundTrips() throws Exception {
        ObjectMapper mapper = mapper();
        UUID postId = UUID.randomUUID();

        FeedService.CachedHomeFeed cached = FeedService.CachedHomeFeed.fromPage(
                List.of(samplePost(postId)), Optional.empty());

        JsonNode node = mapper.readTree(mapper.writeValueAsString(cached));
        assertThat(node.has("next_cursor_nanos")).isFalse();
        assertThat(node.has("next_cursor_id")).isFalse();

        FeedService.FeedPage page = mapper
                .readValue(mapper.writeValueAsString(cached), FeedService.CachedHomeFeed.class)
                .toPage();
        assertThat(page.nextCursor()).isEmpty();
    }

    @Test
    void toPageReturnsNullWhenPostsMissing() {
        FeedService.CachedHomeFeed cached = new FeedService.CachedHomeFeed();
        assertThat(cached.toPage()).isNull();
    }
}
