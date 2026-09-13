package com.ciel.web.dto;

import lombok.AllArgsConstructor;
import lombok.Data;
import lombok.NoArgsConstructor;

import java.util.List;
import java.util.UUID;

@Data
@NoArgsConstructor
@AllArgsConstructor
public class UploadIntent {
    private UUID uploadId;
    private String objectKey;
    private String uploadUrl;
    private long expiresInSeconds;
    private List<UploadHeader> headers;
}
