package com.ciel.web;

import com.ciel.config.AppProperties;
import com.ciel.web.error.ApiException;
import jakarta.servlet.http.HttpServletRequest;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.RestController;

@RestController
public class MetricsController {
  private final AppProperties config;
  private final RequestMetrics metrics;
  public MetricsController(AppProperties config, RequestMetrics metrics) {
    this.config = config;
    this.metrics = metrics;
  }
  @GetMapping(value = "/metrics", produces = "text/plain;charset=UTF-8")
  public String metrics(HttpServletRequest request) {
    String expected = config.getAdminToken();
    if (expected == null || expected.isEmpty())
      throw ApiException.forbidden("admin token not configured");
    String provided = request.getHeader("x-admin-token");
    if (provided == null)
      throw ApiException.forbidden("missing admin token");
    if (!MessageDigest.isEqual(expected.getBytes(StandardCharsets.UTF_8), provided.getBytes(StandardCharsets.UTF_8)))
      throw ApiException.forbidden("invalid admin token");
    return metrics.render();
  }
}
