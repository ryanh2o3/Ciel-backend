package com.ciel.web;

import com.ciel.app.AuthService;
import com.ciel.app.ExtendedApiService;
import com.ciel.config.AppProperties;
import com.ciel.web.auth.AuthUser;
import com.ciel.web.auth.BearerAuth;
import com.ciel.web.error.ApiException;
import jakarta.servlet.http.HttpServletRequest;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.util.*;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.*;

@RestController
@RequestMapping("/v1")
public class ExtendedController {
  private final ExtendedApiService api;
  private final AuthService auth;
  private final AppProperties config;
  public ExtendedController(ExtendedApiService api, AuthService auth, AppProperties config) {
    this.api = api;
    this.auth = auth;
    this.config = config;
  }
  private UUID optional(HttpServletRequest request) {
    return BearerAuth.extractToken(request.getHeader("Authorization"))
        .flatMap(auth::authenticateAccessToken)
        .orElse(null);
  }
  private void admin(HttpServletRequest request) {
    String expected = config.getAdminToken();
    if (expected == null || expected.isEmpty())
      throw ApiException.forbidden("admin token not configured");
    String provided = request.getHeader("x-admin-token");
    if (provided == null)
      throw ApiException.forbidden("missing admin token");
    if (!MessageDigest.isEqual(expected.getBytes(StandardCharsets.UTF_8), provided.getBytes(StandardCharsets.UTF_8)))
      throw ApiException.forbidden("invalid admin token");
  }
  static String s(Map<String, Object> body, String key) {
    Object value = body.get(key);
    if (value == null)
      return null;
    if (!(value instanceof String))
      throw ApiException.badRequest("invalid request body");
    return (String) value;
  }
  static String required(Map<String, Object> body, String key) {
    String value = s(body, key);
    if (value == null)
      throw ApiException.badRequest("invalid request body");
    return value;
  }
  static UUID uuidBody(Map<String, Object> body, String key) {
    try {
      return UUID.fromString(required(body, key));
    } catch (IllegalArgumentException e) {
      throw ApiException.badRequest("invalid request body");
    }
  }
  static long days(Map<String, Object> body) {
    Object value = body.getOrDefault("days_valid", 7);
    if (!(value instanceof Integer || value instanceof Long))
      throw ApiException.badRequest("invalid request body");
    return ((Number) value).longValue();
  }
  @PatchMapping("/users/{id}")
  public Object updateProfile(
      HttpServletRequest request, AuthUser identity, @PathVariable UUID id, @RequestBody Map<String, Object> body) {
    UUID user = identity.userId();
    return api.updateProfile(user, id, s(body, "display_name"), s(body, "bio"), s(body, "avatar_key"));
  }
  @DeleteMapping("/account")
  public Object deleteAccount(HttpServletRequest request, AuthUser identity) {
    UUID user = identity.userId();
    api.deleteAccount(user);
    return ResponseEntity.noContent().build();
  }
  @GetMapping("/users/{id}/posts")
  public Object userPosts(HttpServletRequest request, @PathVariable UUID id,
      @RequestParam(required = false) Integer limit, @RequestParam(required = false) String cursor) {
    UUID user = optional(request);
    return api.userPosts(id, user, limit, cursor);
  }
  @PostMapping("/users/{id}/follow")
  public Object follow(HttpServletRequest request, AuthUser identity, @PathVariable UUID id) {
    UUID user = identity.userId();
    return api.social(user, id, "follow");
  }
  @PostMapping("/users/{id}/unfollow")
  public Object unfollow(HttpServletRequest request, AuthUser identity, @PathVariable UUID id) {
    UUID user = identity.userId();
    return api.social(user, id, "unfollow");
  }
  @PostMapping("/users/{id}/block")
  public Object block(HttpServletRequest request, AuthUser identity, @PathVariable UUID id) {
    UUID user = identity.userId();
    return api.social(user, id, "block");
  }
  @PostMapping("/users/{id}/unblock")
  public Object unblock(HttpServletRequest request, AuthUser identity, @PathVariable UUID id) {
    UUID user = identity.userId();
    return api.social(user, id, "unblock");
  }
  @GetMapping("/users/{id}/relationship")
  public Object relationship(HttpServletRequest request, AuthUser identity, @PathVariable UUID id) {
    UUID user = identity.userId();
    return api.relationship(user, id);
  }
  @GetMapping("/users/{id}/followers")
  public Object socialList(HttpServletRequest request, AuthUser identity, @PathVariable UUID id,
      @RequestParam(required = false) Integer limit, @RequestParam(required = false) String cursor) {
    UUID user = identity.userId();
    return api.socialList(id, true, limit, cursor);
  }
  @GetMapping("/users/{id}/following")
  public Object socialListRoute(HttpServletRequest request, AuthUser identity, @PathVariable UUID id,
      @RequestParam(required = false) Integer limit, @RequestParam(required = false) String cursor) {
    UUID user = identity.userId();
    return api.socialList(id, false, limit, cursor);
  }
  @PatchMapping("/posts/{id}")
  public Object updatePost(
      HttpServletRequest request, AuthUser identity, @PathVariable UUID id, @RequestBody Map<String, Object> body) {
    UUID user = identity.userId();
    return api.updatePost(user, id, s(body, "caption"));
  }
  @DeleteMapping("/posts/{id}")
  public Object deletePost(HttpServletRequest request, AuthUser identity, @PathVariable UUID id) {
    UUID user = identity.userId();
    api.deletePost(user, id);
    return ResponseEntity.noContent().build();
  }
  @GetMapping("/posts/{id}/likes")
  public Object engagements(HttpServletRequest request, AuthUser identity, @PathVariable UUID id,
      @RequestParam(required = false) Integer limit, @RequestParam(required = false) String cursor) {
    UUID user = identity.userId();
    return api.engagements(user, id, true, limit, cursor);
  }
  @GetMapping("/posts/{id}/comments")
  public Object engagementsRoute(HttpServletRequest request, AuthUser identity, @PathVariable UUID id,
      @RequestParam(required = false) Integer limit, @RequestParam(required = false) String cursor) {
    UUID user = identity.userId();
    return api.engagements(user, id, false, limit, cursor);
  }
  @DeleteMapping("/posts/{id}/comments/{comment_id}")
  public Object deleteComment(HttpServletRequest request, AuthUser identity, @PathVariable UUID id,
      @PathVariable("comment_id") UUID commentId) {
    UUID user = identity.userId();
    api.deleteComment(user, id, commentId);
    return ResponseEntity.noContent().build();
  }
  @DeleteMapping("/media/{id}")
  public Object deleteMedia(HttpServletRequest request, AuthUser identity, @PathVariable UUID id) {
    UUID user = identity.userId();
    api.deleteMedia(user, id);
    return ResponseEntity.noContent().build();
  }
  @GetMapping("/notifications")
  public Object notifications(HttpServletRequest request, AuthUser identity,
      @RequestParam(required = false) Integer limit, @RequestParam(required = false) String cursor) {
    UUID user = identity.userId();
    return api.notifications(user, limit, cursor);
  }
  @PostMapping("/notifications/{id}/read")
  public Object markNotification(HttpServletRequest request, AuthUser identity, @PathVariable UUID id) {
    UUID user = identity.userId();
    api.markNotification(user, id);
    return ResponseEntity.noContent().build();
  }
  @GetMapping("/search/users")
  public Object search(HttpServletRequest request, AuthUser identity, @RequestParam(required = false) Integer limit,
      @RequestParam(required = false) String cursor, @RequestParam String q) {
    UUID user = identity.userId();
    return api.search(true, q, limit, cursor);
  }
  @GetMapping("/search/posts")
  public Object searchRoute(HttpServletRequest request, AuthUser identity,
      @RequestParam(required = false) Integer limit, @RequestParam(required = false) String cursor,
      @RequestParam String q) {
    UUID user = identity.userId();
    return api.search(false, q, limit, cursor);
  }
  @PostMapping("/moderation/users/{id}/flag")
  public Object flag(
      HttpServletRequest request, AuthUser identity, @PathVariable UUID id, @RequestBody Map<String, Object> body) {
    UUID user = identity.userId();
    return api.flag(user, id, false, s(body, "reason"));
  }
  @PostMapping("/moderation/posts/{id}/flag")
  public Object flagRoute(
      HttpServletRequest request, AuthUser identity, @PathVariable UUID id, @RequestBody Map<String, Object> body) {
    UUID user = identity.userId();
    return api.flag(user, id, true, s(body, "reason"));
  }
  @PostMapping("/moderation/posts/{id}/takedown")
  public Object takedown(HttpServletRequest request, @PathVariable UUID id, @RequestBody Map<String, Object> body) {
    UUID user = optional(request);
    admin(request);
    api.takedown(user, id, true, s(body, "reason"));
    return ResponseEntity.noContent().build();
  }
  @PostMapping("/moderation/comments/{id}/takedown")
  public Object takedownRoute(
      HttpServletRequest request, @PathVariable UUID id, @RequestBody Map<String, Object> body) {
    UUID user = optional(request);
    admin(request);
    api.takedown(user, id, false, s(body, "reason"));
    return ResponseEntity.noContent().build();
  }
  @GetMapping("/moderation/audit")
  public Object audit(HttpServletRequest request, @RequestParam(required = false) Integer limit,
      @RequestParam(required = false) String cursor) {
    UUID user = optional(request);
    admin(request);
    return api.audit(limit, cursor);
  }
  @GetMapping("/account/trust-score")
  public Object trust(HttpServletRequest request, AuthUser identity) {
    UUID user = identity.userId();
    return api.trust(user);
  }
  @GetMapping("/account/rate-limits")
  public Object quotas(HttpServletRequest request, AuthUser identity) {
    UUID user = identity.userId();
    return api.quotas(user);
  }
  @PostMapping("/account/device/register")
  public Object registerDevice(HttpServletRequest request, @RequestBody Map<String, Object> body) {
    UUID user = optional(request);
    api.registerDevice(user, s(body, "fingerprint"), request.getHeader("User-Agent"));
    return ResponseEntity.noContent().build();
  }
  @GetMapping("/account/devices")
  public Object devices(HttpServletRequest request, AuthUser identity) {
    UUID user = identity.userId();
    return api.devices(user);
  }
  @GetMapping("/invites")
  public Object invites(HttpServletRequest request, AuthUser identity) {
    UUID user = identity.userId();
    return api.invites(user);
  }
  @PostMapping("/invites")
  public Object createInvite(HttpServletRequest request, AuthUser identity, @RequestBody Map<String, Object> body) {
    UUID user = identity.userId();
    return api.createInvite(user, days(body));
  }
  @GetMapping("/invites/stats")
  public Object inviteStats(HttpServletRequest request, AuthUser identity) {
    UUID user = identity.userId();
    return api.inviteStats(user);
  }
  @PostMapping("/invites/{code}/revoke")
  public Object revokeInvite(HttpServletRequest request, AuthUser identity, @PathVariable String code) {
    UUID user = identity.userId();
    api.revokeInvite(user, code);
    return ResponseEntity.noContent().build();
  }
  @GetMapping("/invites/validate/{code}")
  public Object validateInvite(HttpServletRequest request, @PathVariable String code) {
    UUID user = null;
    return api.validateInvite(code);
  }
  @PostMapping("/admin/invites")
  public Object createInviteRoute(HttpServletRequest request, @RequestBody Map<String, Object> body) {
    UUID user = optional(request);
    admin(request);
    return api.createInvite(null, days(body));
  }
  @PostMapping("/stories")
  public Object createStory(HttpServletRequest request, AuthUser identity, @RequestBody Map<String, Object> body) {
    UUID user = identity.userId();
    return api.createStory(user, uuidBody(body, "media_id"), s(body, "caption"), required(body, "visibility"));
  }
  @GetMapping("/users/{id}/stories")
  public Object stories(HttpServletRequest request, AuthUser identity, @PathVariable UUID id,
      @RequestParam(required = false) Integer limit, @RequestParam(required = false) String cursor) {
    UUID user = identity.userId();
    return api.stories(user, id, limit, cursor);
  }
  @GetMapping("/stories/{id}")
  public Object getStory(HttpServletRequest request, AuthUser identity, @PathVariable UUID id) {
    UUID user = identity.userId();
    return api.getStory(user, id);
  }
  @DeleteMapping("/stories/{id}")
  public Object deleteStory(HttpServletRequest request, AuthUser identity, @PathVariable UUID id) {
    UUID user = identity.userId();
    api.deleteStory(user, id);
    return ResponseEntity.noContent().build();
  }
  @GetMapping("/stories/{id}/viewers")
  public Object viewers(HttpServletRequest request, AuthUser identity, @PathVariable UUID id,
      @RequestParam(required = false) Integer limit, @RequestParam(required = false) String cursor) {
    UUID user = identity.userId();
    return api.viewers(user, id, limit, cursor);
  }
  @PostMapping("/stories/{id}/reactions")
  public Object react(
      HttpServletRequest request, AuthUser identity, @PathVariable UUID id, @RequestBody Map<String, Object> body) {
    UUID user = identity.userId();
    return api.react(user, id, required(body, "emoji"));
  }
  @GetMapping("/stories/{id}/reactions")
  public Object reactions(HttpServletRequest request, AuthUser identity, @PathVariable UUID id,
      @RequestParam(required = false) Integer limit, @RequestParam(required = false) String cursor) {
    UUID user = identity.userId();
    return api.reactions(user, id, limit, cursor);
  }
  @DeleteMapping("/stories/{id}/reactions")
  public Object removeReaction(HttpServletRequest request, AuthUser identity, @PathVariable UUID id) {
    UUID user = identity.userId();
    api.removeReaction(user, id);
    return ResponseEntity.noContent().build();
  }
  @PostMapping("/stories/{id}/seen")
  public Object seen(HttpServletRequest request, AuthUser identity, @PathVariable UUID id) {
    UUID user = identity.userId();
    api.seen(user, id);
    return ResponseEntity.noContent().build();
  }
  @GetMapping("/feed/stories")
  public Object storiesRoute(HttpServletRequest request, AuthUser identity,
      @RequestParam(required = false) Integer limit, @RequestParam(required = false) String cursor) {
    UUID user = identity.userId();
    return api.stories(user, null, limit, cursor);
  }
  @GetMapping("/stories/{id}/metrics")
  public Object metrics(HttpServletRequest request, AuthUser identity, @PathVariable UUID id) {
    UUID user = identity.userId();
    return api.metrics(user, id);
  }
  @PostMapping("/stories/{id}/highlights")
  public Object highlight(
      HttpServletRequest request, AuthUser identity, @PathVariable UUID id, @RequestBody Map<String, Object> body) {
    UUID user = identity.userId();
    return api.highlight(user, id, required(body, "highlight_name"));
  }
  @GetMapping("/users/{id}/highlights")
  public Object highlights(HttpServletRequest request, AuthUser identity, @PathVariable UUID id,
      @RequestParam(required = false) Integer limit, @RequestParam(required = false) String cursor) {
    UUID user = identity.userId();
    return api.highlights(id, limit, cursor);
  }
}
