package com.ciel.web.dto;

import lombok.Data;

@Data
public class UploadRequest {
    private String contentType;
    private long bytes;
}
