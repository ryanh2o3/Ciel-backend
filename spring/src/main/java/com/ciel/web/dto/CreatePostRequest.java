package com.ciel.web.dto;

import lombok.Data;

import java.util.List;
import java.util.UUID;

@Data
public class CreatePostRequest {
    private List<UUID> mediaIds;
    private String caption;
}
