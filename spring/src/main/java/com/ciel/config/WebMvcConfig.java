package com.ciel.config;

import com.ciel.web.auth.AuthUserResolver;
import org.springframework.context.annotation.Configuration;
import org.springframework.web.method.support.HandlerMethodArgumentResolver;
import org.springframework.web.servlet.config.annotation.WebMvcConfigurer;

import java.util.List;

/**
 * Web MVC wiring only — kept separate from {@link InfraConfig} so AWS/ObjectMapper
 * beans do not participate in the AuthUserResolver ↔ AuthService cycle.
 */
@Configuration
public class WebMvcConfig implements WebMvcConfigurer {

    private final AuthUserResolver authUserResolver;

    public WebMvcConfig(AuthUserResolver authUserResolver) {
        this.authUserResolver = authUserResolver;
    }

    @Override
    public void addArgumentResolvers(List<HandlerMethodArgumentResolver> resolvers) {
        resolvers.add(authUserResolver);
    }
}
