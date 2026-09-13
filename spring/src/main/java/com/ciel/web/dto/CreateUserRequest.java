package com.ciel.web.dto;

import lombok.Data;

@Data
public class CreateUserRequest {
    private String handle;
    private String email;
    private String displayName;
    private String bio;
    private String avatarKey;
    private String password;
    private String inviteCode;
}
