package com.ciel.web.auth;

import com.ciel.app.AuthService;
import com.ciel.web.error.ApiException;
import jakarta.servlet.http.HttpServletRequest;
import org.springframework.core.MethodParameter;
import org.springframework.stereotype.Component;
import org.springframework.web.bind.support.WebDataBinderFactory;
import org.springframework.web.context.request.NativeWebRequest;
import org.springframework.web.method.support.HandlerMethodArgumentResolver;
import org.springframework.web.method.support.ModelAndViewContainer;

import java.util.UUID;

@Component
public class AuthUserResolver implements HandlerMethodArgumentResolver {

    private final AuthService authService;

    public AuthUserResolver(AuthService authService) {
        this.authService = authService;
    }

    @Override
    public boolean supportsParameter(MethodParameter parameter) {
        return parameter.getParameterType().equals(AuthUser.class);
    }

    @Override
    public Object resolveArgument(
            MethodParameter parameter,
            ModelAndViewContainer mavContainer,
            NativeWebRequest webRequest,
            WebDataBinderFactory binderFactory) {
        HttpServletRequest request = webRequest.getNativeRequest(HttpServletRequest.class);
        String header = request != null ? request.getHeader("Authorization") : null;

        if (header == null || !header.regionMatches(true, 0, "Bearer ", 0, 7)) {
            throw ApiException.unauthorized("missing Authorization header");
        }
        String token = header.substring(7).trim();
        UUID userId = authService
                .authenticateAccessToken(token)
                .orElseThrow(() -> ApiException.unauthorized("invalid token"));
        return new AuthUser(userId);
    }
}
