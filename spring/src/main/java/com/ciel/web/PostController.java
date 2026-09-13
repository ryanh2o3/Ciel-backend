package com.ciel.web;

import com.ciel.app.AuthService;
import com.ciel.app.FeedService;
import com.ciel.app.MediaService;
import com.ciel.app.PostService;
import com.ciel.domain.Comment;
import com.ciel.domain.Post;
import com.ciel.web.auth.AuthUser;
import com.ciel.web.auth.BearerAuth;
import com.ciel.web.dto.CommentRequest;
import com.ciel.web.dto.CreatePostRequest;
import com.ciel.web.dto.LikeResponse;
import com.ciel.web.error.ApiException;
import org.springframework.http.HttpStatus;
import org.springframework.web.bind.annotation.DeleteMapping;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.PathVariable;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RequestHeader;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.ResponseStatus;
import org.springframework.web.bind.annotation.RestController;

import java.util.Optional;
import java.util.UUID;

@RestController
@RequestMapping("/v1/posts")
public class PostController {

    private final PostService postService;
    private final MediaService mediaService;
    private final FeedService feedService;
    private final AuthService authService;

    public PostController(
            PostService postService,
            MediaService mediaService,
            FeedService feedService,
            AuthService authService) {
        this.postService = postService;
        this.mediaService = mediaService;
        this.feedService = feedService;
        this.authService = authService;
    }

    @PostMapping
    public Post createPost(AuthUser auth, @RequestBody CreatePostRequest body) {
        if (body.getMediaIds() == null || body.getMediaIds().isEmpty()) {
            throw ApiException.badRequest("at least one media_id is required");
        }
        if (body.getMediaIds().size() > 10) {
            throw ApiException.badRequest("maximum 10 images per post");
        }
        RequestValidation.validateMaxLen("caption", body.getCaption(), RequestValidation.MAX_CAPTION_LEN);
        Post post = postService.createPost(auth.userId(), body.getMediaIds(), body.getCaption());
        feedService.refreshHomeFeed(auth.userId());
        mediaService.populatePostAvatarUrls(java.util.List.of(post));
        return post;
    }

    @GetMapping("/{id}")
    public Post getPost(
            @PathVariable UUID id,
            @RequestHeader(value = "Authorization", required = false) String authorization) {
        Optional<UUID> viewer = optionalViewer(authorization);
        Post post = postService
                .getPost(id, viewer)
                .orElseThrow(() -> ApiException.notFound("post not found"));
        mediaService.populatePostAvatarUrls(java.util.List.of(post));
        return post;
    }

    @PostMapping("/{id}/like")
    public LikeResponse likePost(@PathVariable UUID id, AuthUser auth) {
        ensureVisible(id, auth.userId());
        boolean created = postService.likePost(auth.userId(), id).isPresent();
        return new LikeResponse(created);
    }

    @DeleteMapping("/{id}/like")
    @ResponseStatus(HttpStatus.NO_CONTENT)
    public void unlikePost(@PathVariable UUID id, AuthUser auth) {
        if (!postService.unlikePost(auth.userId(), id)) {
            throw ApiException.notFound("like not found");
        }
    }

    @PostMapping("/{id}/comment")
    public Comment commentPost(@PathVariable UUID id, AuthUser auth, @RequestBody CommentRequest body) {
        if (body.getBody() == null || body.getBody().trim().isEmpty()) {
            throw ApiException.badRequest("comment body cannot be empty");
        }
        if (body.getBody().length() > RequestValidation.MAX_COMMENT_LEN) {
            throw ApiException.badRequest("comment body exceeds 1000 characters");
        }
        ensureVisible(id, auth.userId());
        return postService.addComment(auth.userId(), id, body.getBody());
    }

    private void ensureVisible(UUID postId, UUID viewerId) {
        if (!postService.canViewPost(postId, viewerId)) {
            throw ApiException.notFound("post not found");
        }
    }

    private Optional<UUID> optionalViewer(String authorization) {
        return BearerAuth.extractToken(authorization).flatMap(authService::authenticateAccessToken);
    }
}
