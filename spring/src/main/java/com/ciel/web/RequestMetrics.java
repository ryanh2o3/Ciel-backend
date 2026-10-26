package com.ciel.web;

import jakarta.servlet.FilterChain;
import jakarta.servlet.ServletException;
import jakarta.servlet.http.HttpServletRequest;
import jakarta.servlet.http.HttpServletResponse;
import java.io.IOException;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.DoubleAdder;
import java.util.concurrent.atomic.LongAdder;
import org.springframework.stereotype.Component;
import org.springframework.web.filter.OncePerRequestFilter;
import org.springframework.web.servlet.HandlerMapping;

@Component
public class RequestMetrics extends OncePerRequestFilter {
  private record Key(String method, String path, int status) {}
  private static class Sample {
    final LongAdder count = new LongAdder();
    final DoubleAdder seconds = new DoubleAdder();
  }
  private final ConcurrentHashMap<Key, Sample> samples = new ConcurrentHashMap<>();
  @Override
  protected void doFilterInternal(HttpServletRequest request, HttpServletResponse response, FilterChain chain)
      throws ServletException, IOException {
    long start = System.nanoTime();
    try {
      chain.doFilter(request, response);
    } finally {
      Object pattern = request.getAttribute(HandlerMapping.BEST_MATCHING_PATTERN_ATTRIBUTE);
      String path = pattern == null ? "unmatched" : pattern.toString().replaceAll("\\{([^}]+)\\}", ":$1");
      var sample = samples.computeIfAbsent(new Key(request.getMethod(), path, response.getStatus()), k -> new Sample());
      sample.count.increment();
      sample.seconds.add((System.nanoTime() - start) / 1_000_000_000.0);
    }
  }
  public String render() {
    var text = new StringBuilder("# TYPE http_requests_total counter\n# TYPE http_request_duration_seconds summary\n");
    samples.forEach((key, sample) -> {
      String labels = "{method=\"" + key.method() + "\",path=\"" + key.path() + "\",status=\"" + key.status() + "\"}";
      text.append("http_requests_total").append(labels).append(' ').append(sample.count.sum()).append('\n');
      text.append("http_request_duration_seconds_count")
          .append(labels)
          .append(' ')
          .append(sample.count.sum())
          .append('\n');
      text.append("http_request_duration_seconds_sum")
          .append(labels)
          .append(' ')
          .append(sample.seconds.sum())
          .append('\n');
    });
    return text.toString();
  }
}
