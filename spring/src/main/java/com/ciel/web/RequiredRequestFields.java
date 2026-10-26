package com.ciel.web;

import com.ciel.web.error.ApiException;
import java.lang.reflect.Type;
import java.util.Set;
import org.springframework.core.MethodParameter;
import org.springframework.http.HttpInputMessage;
import org.springframework.http.converter.HttpMessageConverter;
import org.springframework.web.bind.annotation.ControllerAdvice;
import org.springframework.web.servlet.mvc.method.annotation.RequestBodyAdviceAdapter;

/** Reject absent/null required DTO fields before business validation, like serde. */
@ControllerAdvice
public class RequiredRequestFields extends RequestBodyAdviceAdapter {
    private static final Set<String> OPTIONAL = Set.of("bio", "avatarKey", "caption");

    @Override
    public boolean supports(MethodParameter parameter, Type type,
            Class<? extends HttpMessageConverter<?>> converter) {
        return parameter.getParameterType().getPackageName().equals("com.ciel.web.dto");
    }

    @Override
    public Object afterBodyRead(Object body, HttpInputMessage input, MethodParameter parameter,
            Type type, Class<? extends HttpMessageConverter<?>> converter) {
        for (var field : body.getClass().getDeclaredFields()) {
            if (OPTIONAL.contains(field.getName()) || java.lang.reflect.Modifier.isStatic(field.getModifiers())) continue;
            try {
                field.setAccessible(true);
                if (field.get(body) == null) throw ApiException.badRequest("invalid request body");
            } catch (IllegalAccessException ex) {
                throw new IllegalStateException(ex);
            }
        }
        return body;
    }
}
