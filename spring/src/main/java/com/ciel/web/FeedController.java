package com.ciel.web;

import com.ciel.app.CursorUtil;
import com.ciel.app.FeedService;
import com.ciel.app.MediaService;
import com.ciel.domain.Post;
import com.ciel.web.auth.AuthUser;
import com.ciel.web.dto.ListResponse;
import com.ciel.web.error.ApiException;
import org.springframework.http.HttpStatus;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RequestParam;
import org.springframework.web.bind.annotation.ResponseStatus;
import org.springframework.web.bind.annotation.RestController;

import java.util.ArrayList;
import java.util.List;

@RestController
@RequestMapping("/v1/feed")
public class FeedController {

    private final FeedService feedService;
    private final MediaService mediaService;

    public FeedController(FeedService feedService, MediaService mediaService) {
        this.feedService = feedService;
        this.mediaService = mediaService;
    }

    @GetMapping
    public ListResponse<Post> homeFeed(
            AuthUser auth,
            @RequestParam(required = false) Integer limit,
            @RequestParam(required = false) String cursor) {
        int pageSize = limit == null ? 30 : limit;
        if (pageSize < 1 || pageSize > 200) {
            throw ApiException.badRequest("limit must be between 1 and 200");
        }
        var parsed = CursorUtil.parse(cursor);
        FeedService.FeedPage page = feedService.getHomeFeed(auth.userId(), parsed, pageSize);
        List<Post> posts = new ArrayList<>(page.posts());
        mediaService.populatePostAvatarUrls(posts);
        String next = page.nextCursor()
                .map(c -> CursorUtil.encode(c.timestamp(), c.id()))
                .orElse(null);
        return new ListResponse<>(posts, next);
    }

    @PostMapping("/refresh")
    @ResponseStatus(HttpStatus.NO_CONTENT)
    public void refreshFeed(AuthUser auth) {
        feedService.refreshHomeFeed(auth.userId());
    }
}
