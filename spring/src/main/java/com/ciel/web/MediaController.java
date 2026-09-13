package com.ciel.web;

import com.ciel.app.MediaService;
import com.ciel.domain.Media;
import com.ciel.web.auth.AuthUser;
import com.ciel.web.dto.UploadIntent;
import com.ciel.web.dto.UploadRequest;
import com.ciel.web.dto.UploadStatus;
import com.ciel.web.error.ApiException;
import com.ciel.config.AppProperties;
import org.springframework.http.HttpStatus;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.PathVariable;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.ResponseStatus;
import org.springframework.web.bind.annotation.RestController;

import java.util.UUID;

@RestController
@RequestMapping("/v1/media")
public class MediaController {

    private final MediaService mediaService;
    private final AppProperties props;

    public MediaController(MediaService mediaService, AppProperties props) {
        this.mediaService = mediaService;
        this.props = props;
    }

    @PostMapping("/upload")
    public UploadIntent createUpload(AuthUser auth, @RequestBody UploadRequest body) {
        if (body.getContentType() == null || body.getContentType().isBlank()) {
            throw ApiException.badRequest("invalid upload request");
        }
        if (body.getBytes() <= 0) {
            throw ApiException.badRequest("bytes must be greater than 0");
        }
        if (body.getBytes() > props.getUpload().getMaxBytes()) {
            throw ApiException.badRequest("upload exceeds max size");
        }
        try {
            return mediaService.createUpload(
                    auth.userId(),
                    body.getContentType(),
                    body.getBytes(),
                    props.getUpload().getUrlTtlSeconds());
        } catch (ApiException e) {
            throw e;
        } catch (Exception e) {
            throw ApiException.badRequest("invalid upload request");
        }
    }

    @PostMapping("/upload/{id}/complete")
    @ResponseStatus(HttpStatus.ACCEPTED)
    public void completeUpload(@PathVariable UUID id, AuthUser auth) {
        if (!mediaService.completeUpload(id, auth.userId())) {
            throw ApiException.notFound("upload not found");
        }
    }

    @GetMapping("/upload/{id}/status")
    public UploadStatus uploadStatus(@PathVariable UUID id, AuthUser auth) {
        return mediaService
                .getUploadStatus(id, auth.userId())
                .orElseThrow(() -> ApiException.notFound("upload not found"));
    }

    @GetMapping("/{id}")
    public Media getMedia(@PathVariable UUID id, AuthUser auth) {
        return mediaService
                .getMediaForUser(id, auth.userId())
                .orElseThrow(() -> ApiException.notFound("media not found"));
    }
}
