package com.ciel.app;

import static com.ciel.app.ApiData.*;
import static com.ciel.app.ParitySql.*;

import com.ciel.config.AppProperties;
import com.ciel.config.RateLimits;
import com.ciel.config.TrustLevel;
import com.ciel.web.error.ApiException;
import java.nio.charset.StandardCharsets;
import java.security.SecureRandom;
import java.time.*;
import java.util.*;
import org.springframework.dao.DataIntegrityViolationException;
import org.springframework.data.redis.core.ScanOptions;
import org.springframework.data.redis.core.StringRedisTemplate;
import org.springframework.stereotype.Service;
import software.amazon.awssdk.services.s3.S3Client;

/** Native implementations of social, safety, stories, search and moderation. */
@Service
public class ExtendedApiService {
  private final ApiData db;
  private final MediaService media;
  private final StringRedisTemplate redis;
  private final S3Client storage;
  private final AppProperties config;
  private final CryptoService crypto;
  public ExtendedApiService(ApiData db, MediaService media, StringRedisTemplate redis, S3Client storage,
      AppProperties config, CryptoService crypto) {
    this.db = db;
    this.media = media;
    this.redis = redis;
    this.storage = storage;
    this.config = config;
    this.crypto = crypto;
  }
  static void length(String value, int max, String error) {
    if (value != null && value.getBytes(StandardCharsets.UTF_8).length > max)
      throw ApiException.badRequest(error);
  }
  static void characters(String value, int max, String error) {
    if (value != null && value.codePointCount(0, value.length()) > max)
      throw ApiException.badRequest(error);
  }
  private void changed(int count, String error) {
    if (count == 0)
      throw ApiException.notFound(error);
  }
  private void avatar(Map<String, Object> row, String prefix, boolean optional) {
    String key = text(row, prefix + "avatar_key");
    String url = media.generatePresignedGetUrl(key, 14400).orElse(null);
    if (optional) {
      if (key == null)
        row.remove(prefix + "avatar_key");
      if (url != null)
        row.put(prefix + "avatar_url", url);
    } else {
      row.remove(prefix + "avatar_key");
      row.put(prefix + "avatar_url", url);
    }
  }
  private Map<String, Object> publicUser(Map<String, Object> row, boolean resolve) {
    row.remove("email");
    if (resolve)
      avatar(row, "", false);
    else {
      row.remove("avatar_key");
      row.put("avatar_url", null);
    }
    row.put("followers_count", 0L);
    row.put("following_count", 0L);
    row.put("posts_count", 0L);
    return row;
  }
  private void post(Map<String, Object> row, boolean primary) {
    row.put("visibility", "public".equals(row.get("visibility")) ? "Public" : "FollowersOnly");
    avatar(row, "owner_", true);
    if (primary && row.get("media_ids") instanceof List<?> ids && !ids.isEmpty()) {
      media.getMedia(UUID.fromString(ids.getFirst().toString())).ifPresent(m -> row.put("primary_media", m));
    }
  }
  private void story(Map<String, Object> row) {
    row.put("visibility", switch (text(row, "visibility")) {
      case "public" -> "Public";
      case "friends_only" -> "FriendsOnly";
      case "close_friends_only" -> "CloseFriendsOnly";
      default -> throw ApiException.internal("internal server error");
    });
    avatar(row, "user_", true);
    media.getMedia(uuid(row, "media_id")).ifPresent(m -> row.put("media", m));
  }
  public void invalidate(UUID user) {
    try {
      redis.delete("feed:home:" + user);
      try (var scan = redis.scan(ScanOptions.scanOptions().match("feed:stories:" + user + ":*").count(100).build())) {
        while (scan.hasNext()) redis.delete(scan.next());
      }
    } catch (Exception e) {
      org.slf4j.LoggerFactory.getLogger(getClass()).warn("feed invalidation failed", e);
    }
  }
  public void invalidateFollowers(UUID user) {
    for (var row : db.rows(social_list_follower_ids_0, user)) invalidate(uuid(row, "follower_id"));
    invalidate(user);
  }
  public void activity(UUID user, String type) {
    try {
      int points = switch (type) {
        case "post_created" -> 5;
        case "comment_created" -> 2;
        case "like_received" -> 1;
        case "follower_gained" -> 3;
        case "content_removed" -> -25;
        default -> 0;
      };
      String field = switch (type) {
        case "post_created" -> "posts_count";
        case "comment_created" -> "comments_count";
        case "like_received" -> "likes_received_count";
        case "follower_gained" -> "followers_count";
        default -> null;
      };
      db.execute("UPDATE user_trust_scores SET "
                 + "trust_points=GREATEST(0,trust_points+$1),updated_at=now(),last_activity_at=now()"
              + (field == null ? "" : "," + field + "=" + field + "+1") + " WHERE user_id=$2",
          points, user);
      if (Math.abs(points) >= 5)
        recalculate(user);
    } catch (Exception e) {
      org.slf4j.LoggerFactory.getLogger(getClass()).warn("trust activity failed", e);
    }
  }
  private void recalculate(UUID user) {
    db.execute("UPDATE user_trust_scores SET trust_level=CASE WHEN strikes>=3 THEN 0 WHEN account_age_days>=90 AND "
               + "posts_count>=50 AND trust_points>=200 AND flags_received<3 THEN 2 WHEN account_age_days>=7 AND "
               + "posts_count>=5 AND trust_points>=20 AND flags_received<5 THEN 1 ELSE 0 END,updated_at=now() WHERE "
               + "user_id=$1",
        user);
  }
  public void notify(UUID recipient, UUID actor, String type, Map<String, Object> payload) {
    if (recipient.equals(actor))
      return;
    try {
      var person = db.one(notifications_create_if_not_self_0, actor);
      if (person != null) {
        payload.put("actor_id", actor);
        payload.put("actor_handle", person.get("handle"));
        payload.put("actor_display_name", person.get("display_name"));
        String role = switch (type) {
          case "user_followed" -> "follower";
          case "post_liked" -> "liker";
          case "post_commented" -> "commenter";
          default -> "reactor";
        };
        payload.put(role + "_id", actor);
        payload.put(role + "_handle", person.get("handle"));
      }
      String json = new com.fasterxml.jackson.databind.ObjectMapper().writeValueAsString(payload);
      db.execute("INSERT INTO notifications(user_id,notification_type,payload) VALUES($1,$2,$3::jsonb)", recipient,
          type, json);
    } catch (Exception e) {
      org.slf4j.LoggerFactory.getLogger(getClass()).warn("notification failed", e);
    }
  }
  public Map<String, Object> updateProfile(UUID user, UUID id, String display, String bio, String key) {
    if (!user.equals(id))
      throw ApiException.forbidden("cannot update other users");
    if (display != null && display.isBlank())
      throw ApiException.badRequest("display_name cannot be empty");
    length(display, 50, "display_name must be at most 50 characters");
    length(bio, 500, "bio must be at most 500 characters");
    var previous = db.one(users_get_user_0, id);
    var result = db.required(users_update_profile_0, "user not found", id, display, bio, key);
    if (previous != null && key != null && !key.equals(previous.get("avatar_key"))) {
      try {
        redis.delete("presigned:" + previous.get("avatar_key"));
      } catch (Exception ignored) {
      }
    }
    avatar(result, "", false);
    return result;
  }
  public void deleteAccount(UUID user) {
    db.transaction(() -> {
      changed(db.execute(users_delete_account_0, user), "user not found");
      db.execute(users_delete_account_1, user);
      db.execute(users_delete_account_2, user);
      db.execute(users_delete_account_3, user);
      return null;
    });
  }
  public Map<String, Object> social(UUID user, UUID other, String action) {
    if (user.equals(other))
      throw ApiException.badRequest("cannot " + action + " yourself");
    boolean result = db.transaction(() -> {
      if (action.equals("follow")) {
        db.required(social_follow_0, "user not found", other);
        var count = db.one(social_follow_1, other);
        if (((Number) count.values().iterator().next()).longValue() >= 5000)
          throw ApiException.forbidden("user has reached the follower limit");
        return db.execute(social_follow_2, user, other) > 0;
      }
      if (action.equals("block")) {
        boolean inserted = db.execute(social_block_0, user, other) > 0;
        db.execute(social_block_1, user, other);
        return inserted;
      }
      return db.execute(action.equals("unfollow") ? social_unfollow_0 : social_unblock_0, user, other) > 0;
    });
    if (result) {
      invalidate(user);
      if (action.equals("follow")) {
        notify(other, user, "user_followed", object());
        activity(other, "follower_gained");
      }
    }
    return object(switch (action) {
      case "follow" -> "followed";
      case "unfollow" -> "unfollowed";
      case "block" -> "blocked";
      default -> "unblocked";
    }, result);
  }
  public Map<String, Object> relationship(UUID user, UUID other) {
    if (user.equals(other))
      return object("is_following", false, "is_followed_by", false, "is_blocking", false, "is_blocked_by", false);
    return db.one(social_relationship_status_0, user, other);
  }
  public Map<String, Object> socialList(UUID id, boolean followers, Integer size, String cursor) {
    int n = limit(size);
    var rows = db.rows(followers ? (cursor == null ? social_list_followers_1 : social_list_followers_0)
                                 : (cursor == null ? social_list_following_1 : social_list_following_0),
        cursorArgs(cursor, n, id));
    var result = page(rows, n, "followed_at", "id");
    List<Map<String, Object>> items = new ArrayList<>();
    for (var row : rows) {
      Object followed = row.remove("followed_at");
      items.add(object("user", publicUser(row, false), "followed_at", followed));
    }
    result.put("items", items);
    return result;
  }
  public Map<String, Object> userPosts(UUID id, UUID viewer, Integer size, String cursor) {
    int n = limit(size);
    String sql = viewer == null ? (cursor == null ? posts_list_by_user_3 : posts_list_by_user_2)
                                : (cursor == null ? posts_list_by_user_1 : posts_list_by_user_0);
    Object[] args = viewer == null ? cursorArgs(cursor, n, id) : cursorArgs(cursor, n, id, viewer);
    var rows = db.rows(sql, args);
    var result = page(rows, n, "created_at", "id");
    for (var row : rows) post(row, true);
    return result;
  }
  public Map<String, Object> updatePost(UUID user, UUID id, String caption) {
    length(caption, 2200, "caption must be at most 2200 characters");
    var row = db.required(posts_update_caption_0, "post not found", id, user, caption);
    post(row, false);
    return row;
  }
  public void deletePost(UUID user, UUID id) {
    changed(db.execute(posts_delete_post_0, id, user), "post not found");
  }
  private void visiblePost(UUID user, UUID id) {
    db.required(posts_can_view_post_0, "post not found", id, user);
  }
  public Map<String, Object> engagements(UUID user, UUID id, boolean likes, Integer size, String cursor) {
    int n = limit(size);
    Object[] args = cursorArgs(cursor, n, id);
    visiblePost(user, id);
    return page(db.rows(likes ? (cursor == null ? engagement_list_likes_1 : engagement_list_likes_0)
                              : (cursor == null ? engagement_list_comments_1 : engagement_list_comments_0),
                    args),
        n, "created_at", "id");
  }
  public void deleteComment(UUID user, UUID post, UUID comment) {
    changed(db.execute(engagement_delete_comment_0, comment, post, user), "comment not found");
  }
  public void deleteMedia(UUID user, UUID id) {
    Map<String, Object> row;
    try {
      row =
          db.one("DELETE FROM media WHERE id=$1 AND owner_id=$2 RETURNING original_key,thumb_key,medium_key", id, user);
    } catch (DataIntegrityViolationException e) {
      throw ApiException.conflict("media is attached to a post or story and cannot be deleted");
    }
    if (row == null)
      throw ApiException.notFound("media not found");
    for (Object key : row.values()) {
      try {
        redis.delete("presigned:" + key);
        storage.deleteObject(r -> r.bucket(config.getS3().getBucket()).key(key.toString()));
      } catch (Exception e) {
        org.slf4j.LoggerFactory.getLogger(getClass()).warn("media cleanup failed", e);
      }
    }
  }
  public Map<String, Object> notifications(UUID user, Integer size, String cursor) {
    int n = limit(size);
    return page(db.rows(cursor == null ? notifications_list_1 : notifications_list_0, cursorArgs(cursor, n, user)), n,
        "created_at", "id");
  }
  public void markNotification(UUID user, UUID id) {
    changed(db.execute(notifications_mark_read_0, id, user), "notification not found");
  }
  public Map<String, Object> search(boolean users, String term, Integer size, String cursor) {
    if (term == null)
      throw ApiException.badRequest("missing required parameter 'q'");
    term = term.trim();
    if (term.getBytes(StandardCharsets.UTF_8).length < 2)
      throw ApiException.badRequest("q must be at least 2 characters");
    length(term, 100, "q must be at most 100 characters");
    int n = limit(size);
    String pattern = "%" + term.replace("\\", "\\\\").replace("%", "\\%").replace("_", "\\_") + "%";
    var rows = db.rows(users ? (cursor == null ? search_search_users_1 : search_search_users_0)
                             : (cursor == null ? search_search_posts_1 : search_search_posts_0),
        cursorArgs(cursor, n, pattern));
    var result = page(rows, n, "created_at", "id");
    for (var row : rows) {
      if (users)
        publicUser(row, true);
      else
        post(row, false);
    }
    return result;
  }
  public Map<String, Object> flag(UUID user, UUID id, boolean post, String reason) {
    UUID owner =
        post ? uuid(db.required("SELECT owner_id FROM posts WHERE id=$1", "post not found", id), "owner_id") : id;
    if (user.equals(owner))
      throw ApiException.badRequest(post ? "cannot report your own post" : "cannot report yourself");
    Map<String, Object> result;
    try {
      result = db.transaction(() -> {
        var row = db.one(post ? moderation_flag_post_0 : moderation_flag_user_0, user, id, reason);
        db.execute(post ? moderation_flag_post_1 : moderation_flag_user_1, user, id, reason);
        return row;
      });
    } catch (DataIntegrityViolationException e) {
      Throwable cause = e.getMostSpecificCause();
      String state = cause instanceof java.sql.SQLException sql ? sql.getSQLState() : "";
      if ("23505".equals(state))
        throw ApiException.conflict("you have already reported this " + (post ? "post" : "user"));
      if ("23503".equals(state))
        throw ApiException.notFound(post ? "post not found" : "user not found");
      throw e;
    }
    recordFlag(owner);
    return result;
  }
  private void recordFlag(UUID user) {
    try {
      db.transaction(() -> {
        var row = db.one(trust_record_flag_0, user);
        if (row == null)
          return null;
        int flags = integer(row, "flags_received");
        if (flags >= 10 && flags % 10 == 0) {
          int strikes = integer(db.one(trust_add_strike_with_tx_0, user), "strikes");
          if (strikes >= 3)
            db.execute(trust_add_strike_with_tx_1,
                Instant.now().plus(Duration.ofDays(strikes == 3 ? 7
                        : strikes == 4                          ? 30
                                                                : 365)),
                user);
        }
        recalculate(user);
        return null;
      });
    } catch (Exception e) {
      org.slf4j.LoggerFactory.getLogger(getClass()).warn("flag trust update failed", e);
    }
  }
  public void takedown(UUID actor, UUID id, boolean post, String reason) {
    UUID owner = db.transaction(() -> {
      var row = db.required(post ? moderation_takedown_post_0 : moderation_takedown_comment_0,
          post ? "post not found" : "comment not found", id);
      db.execute(post ? moderation_takedown_post_1 : moderation_takedown_comment_1, actor, id, reason);
      return uuid(row, post ? "owner_id" : "user_id");
    });
    activity(owner, "content_removed");
  }
  public Map<String, Object> audit(Integer size, String cursor) {
    int n = limit(size);
    return page(db.rows(cursor == null ? moderation_list_audit_1 : moderation_list_audit_0, cursorArgs(cursor, n)), n,
        "created_at", "id");
  }
  public Map<String, Object> trust(UUID user) {
    var row = db.required(trust_get_trust_score_0, "trust score not found", user);
    int level = integer(row, "trust_level");
    String name = switch (level) {
      case 1 -> "Basic";
      case 2 -> "Trusted";
      case 3 -> "Verified";
      default -> "New";
    };
    return object("user_id", user, "trust_level", level, "trust_level_name", name, "trust_points",
        row.get("trust_points"), "account_age_days", row.get("account_age_days"), "posts_count", row.get("posts_count"),
        "followers_count", row.get("followers_count"), "strikes", row.get("strikes"), "is_banned",
        row.get("banned_until") != null && instant(row, "banned_until").isAfter(Instant.now()));
  }
  public Map<String, Object> quotas(UUID user) {
    var score = trust(user);
    var limits = RateLimits.forTrustLevel(TrustLevel.fromInt(integer(score, "trust_level")));
    var remaining = object();
    String[] actions = {"post", "follow", "like", "comment", "media_read", "media_upload"};
    String[] fields = {"posts", "follows", "likes", "comments", "media_read", "media_upload"};
    long[] maxima = {limits.getPostsPerHour(), limits.getFollowsPerHour(), limits.getLikesPerHour(),
        limits.getCommentsPerHour(), limits.getMediaReadPerHour(), limits.getMediaUploadPerHour()};
    for (int i = 0; i < actions.length; i++) {
      String count = redis.opsForValue().get(
          "ratelimit:" + user + ":" + actions[i] + ":" + (Instant.now().getEpochSecond() / 3600));
      remaining.put(fields[i], Math.max(0, maxima[i] - (count == null ? 0 : Long.parseLong(count))));
    }
    return object("trust_level", score.get("trust_level_name"), "posts_per_hour", limits.getPostsPerHour(),
        "posts_per_day", limits.getPostsPerDay(), "follows_per_hour", limits.getFollowsPerHour(), "follows_per_day",
        limits.getFollowsPerDay(), "likes_per_hour", limits.getLikesPerHour(), "comments_per_hour",
        limits.getCommentsPerHour(), "media_read_per_hour", limits.getMediaReadPerHour(), "media_upload_per_hour",
        limits.getMediaUploadPerHour(), "remaining", remaining);
  }
  public void registerDevice(UUID user, String fingerprint, String agent) {
    if (fingerprint == null || fingerprint.isBlank())
      throw ApiException.badRequest("fingerprint is required");
    length(fingerprint, 512, "fingerprint must be at most 512 characters");
    String hash = crypto.sha256Hex(fingerprint);
    db.transaction(() -> {
      db.execute(fingerprint_register_fingerprint_0, hash, new UUID[0], agent);
      var row = db.one(fingerprint_register_fingerprint_1, hash);
      if (Boolean.TRUE.equals(row.get("is_blocked")))
        throw ApiException.forbidden("This device has been blocked");
      List<UUID> ids = new ArrayList<>();
      for (Object id : (List<?>) row.get("user_ids")) ids.add(UUID.fromString(id.toString()));
      if (user != null && !ids.contains(user)) {
        int count = integer(row, "account_count");
        int risk = Math.min(100,
            integer(row, "risk_score")
                + (count <= 2         ? 5
                        : count <= 5  ? 15
                        : count <= 10 ? 30
                                      : 50));
        ids.add(user);
        db.execute(fingerprint_register_fingerprint_2, ids.toArray(UUID[] ::new), ids.size(), risk, agent, hash);
      } else
        db.execute(fingerprint_register_fingerprint_3, agent, hash);
      return null;
    });
  }
  public List<Map<String, Object>> devices(UUID user) {
    var rows = db.rows(fingerprint_get_user_devices_0, user);
    for (var row : rows) {
      row.remove("user_ids");
      row.put("fingerprint_hash", text(row, "fingerprint_hash").substring(0, 16));
    }
    return rows;
  }
  private static int maxInvites(int level) {
    return switch (level) {
      case 1 -> 10;
      case 2 -> 50;
      case 3 -> 200;
      default -> 3;
    };
  }
  public List<Map<String, Object>> invites(UUID user) {
    return db.rows(invites_list_user_invites_0, user);
  }
  public Map<String, Object> inviteStats(UUID user) {
    var row = db.required(invites_get_invite_stats_0, "trust score not found", user);
    int max = maxInvites(integer(row, "trust_level"));
    return object("invites_sent", row.get("invites_sent"), "successful_invites", row.get("successful_invites"),
        "remaining_invites", Math.max(0, max - integer(row, "invites_sent")), "max_invites", max);
  }
  public Map<String, Object> validateInvite(String code) {
    var row = db.one(invites_validate_invite_0, code);
    return object("is_valid",
        row != null && Boolean.TRUE.equals(row.get("is_valid")) && instant(row, "expires_at").isAfter(Instant.now())
            && integer(row, "use_count") < integer(row, "max_uses"));
  }
  public Map<String, Object> createInvite(UUID user, long days) {
    int maxDays = user == null ? 365 : 30;
    if (days < 1 || days > maxDays)
      throw ApiException.badRequest("days_valid must be between 1 and " + maxDays);
    return db.transaction(() -> {
      if (user != null) {
        var quota = db.one(invites_create_invite_0, user);
        if (quota == null)
          throw ApiException.internal("internal server error");
        int max = maxInvites(integer(quota, "trust_level"));
        if (integer(quota, "invites_sent") >= max)
          throw ApiException.forbidden("Maximum invite limit reached for your trust level (" + max + ")");
      }
      var random = new SecureRandom();
      String alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
      for (int attempt = 0; attempt < 10; attempt++) {
        var code = new StringBuilder();
        for (int i = 0; i < 12; i++) code.append(alphabet.charAt(random.nextInt(alphabet.length())));
        var row =
            db.one("INSERT INTO invite_codes(code,created_by,expires_at,invite_type) VALUES($1,$2,$3,$4) ON CONFLICT "
                   + "DO NOTHING RETURNING "
                   + "code,created_by,used_by,created_at,used_at,expires_at,is_valid,invite_type,use_count,max_uses",
                code.toString(), user, Instant.now().plus(Duration.ofDays(days)), user == null ? "admin" : "standard");
        if (row != null) {
          if (user != null)
            db.execute(invites_create_invite_2, user);
          return row;
        }
      }
      throw ApiException.internal("internal server error");
    });
  }
  public void revokeInvite(UUID user, String code) {
    changed(db.execute(invites_revoke_invite_0, code, user), "invite code not found or already used");
  }
  public Map<String, Object> createStory(UUID user, UUID mediaId, String caption, String visibility) {
    characters(caption, 2200, "caption exceeds 2200 characters");
    String value = switch (visibility) {
      case "Public" -> "public";
      case "FriendsOnly" -> "friends_only";
      case "CloseFriendsOnly" -> "close_friends_only";
      default -> throw ApiException.badRequest("invalid request body");
    };
    var owner = db.required(stories_create_story_0, "media not found", mediaId);
    if (!user.equals(uuid(owner, "owner_id")))
      throw ApiException.forbidden("media does not belong to you");
    var row = db.one(stories_create_story_1, user, mediaId, caption, Instant.now().plus(Duration.ofHours(24)), value);
    story(row);
    invalidateFollowers(user);
    return row;
  }
  private Map<String, Object> visibleStory(UUID user, UUID id) {
    return db.required(stories_get_story_0, "story not found", id, Instant.now(), user);
  }
  public Map<String, Object> getStory(UUID user, UUID id) {
    var row = visibleStory(user, id);
    story(row);
    return row;
  }
  public Map<String, Object> stories(UUID user, UUID owner, Integer size, String cursor) {
    int n = limit(size);
    List<Map<String, Object>> rows;
    if (owner != null)
      rows = db.rows(cursor == null ? stories_get_user_stories_1 : stories_get_user_stories_0,
          cursorArgs(cursor, n, owner, Instant.now(), user));
    else
      rows = db.rows(cursor == null ? stories_get_stories_feed_1 : stories_get_stories_feed_0,
          cursorArgs(cursor, n, Instant.now(), user));
    var result = page(rows, n, "created_at", "id");
    for (var row : rows) story(row);
    return result;
  }
  public void deleteStory(UUID user, UUID id) {
    changed(db.execute(stories_delete_story_0, id, user), "story not found");
    invalidateFollowers(user);
  }
  public Map<String, Object> viewers(UUID user, UUID id, Integer size, String cursor) {
    int n = limit(size);
    Object[] args = cursorArgs(cursor, n, id);
    var owner = db.required(stories_get_story_owner_0, "story not found", id);
    if (!user.equals(uuid(owner, "user_id")))
      throw ApiException.forbidden("only the story owner can view viewers");
    var rows = db.rows(cursor == null ? stories_list_viewers_1 : stories_list_viewers_0, args);
    var result = page(rows, n, "viewed_at", "viewer_id");
    for (var row : rows) avatar(row, "viewer_", true);
    return result;
  }
  public Map<String, Object> react(UUID user, UUID id, String emoji) {
    if (emoji == null || emoji.isBlank())
      throw ApiException.badRequest("emoji cannot be empty");
    emoji = emoji.trim();
    characters(emoji, 7, "emoji must be at most 7 characters");
    var owner = visibleStory(user, id);
    var row = db.one(stories_add_reaction_0, id, user, emoji);
    row.remove("is_new");
    notify(uuid(owner, "user_id"), user, "story_reaction", object("story_id", id, "emoji", emoji));
    return row;
  }
  public Map<String, Object> reactions(UUID user, UUID id, Integer size, String cursor) {
    int n = limit(size);
    Object[] args = cursorArgs(cursor, n, id);
    visibleStory(user, id);
    return page(
        db.rows(cursor == null ? stories_list_reactions_1 : stories_list_reactions_0, args), n, "created_at", "id");
  }
  public void removeReaction(UUID user, UUID id) {
    changed(db.execute(stories_remove_reaction_0, id, user), "reaction not found");
  }
  public void seen(UUID user, UUID id) {
    visibleStory(user, id);
    db.execute(stories_mark_seen_0, id, user);
  }
  public Map<String, Object> metrics(UUID user, UUID id) {
    var row = db.required(stories_get_metrics_0, "story not found", id, user);
    row.put("story_id", id);
    row.put("reactions_by_emoji", db.rows(stories_get_metrics_1, id));
    row.put("viewer_ids", db.rows(stories_get_metrics_2, id).stream().map(r -> r.get("viewer_id")).toList());
    return row;
  }
  public Map<String, Object> highlight(UUID user, UUID id, String name) {
    if (name == null || name.isBlank())
      throw ApiException.badRequest("highlight_name cannot be empty");
    characters(name, 50, "highlight_name must be at most 50 characters");
    return db.transaction(() -> {
      var owner = db.required(stories_add_to_highlight_0, "story not found", id);
      if (!user.equals(uuid(owner, "user_id")))
        throw ApiException.notFound("story not found");
      var row = db.one(stories_add_to_highlight_1, user, name, id);
      db.execute(stories_add_to_highlight_2, uuid(row, "id"), id);
      return row;
    });
  }
  public Map<String, Object> highlights(UUID user, Integer size, String cursor) {
    int n = limit(size);
    return page(db.rows(cursor == null ? stories_get_user_highlights_1 : stories_get_user_highlights_0,
                    cursorArgs(cursor, n, user)),
        n, "created_at", "id");
  }
  public void postEngagement(UUID user, UUID id, String type, String body) {
    var owner = db.one("SELECT owner_id FROM posts WHERE id=$1", id);
    if (owner == null)
      return;
    var payload = object("post_id", id);
    if (body != null)
      payload.put("comment_preview",
          body.substring(0, body.offsetByCodePoints(0, Math.min(100, body.codePointCount(0, body.length())))));
    notify(uuid(owner, "owner_id"), user, type, payload);
    if (type.equals("post_liked"))
      activity(uuid(owner, "owner_id"), "like_received");
    else
      activity(user, "comment_created");
  }
}
