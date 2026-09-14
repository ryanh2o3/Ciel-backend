package com.ciel.config;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.PropertyNamingStrategies;
import com.fasterxml.jackson.datatype.jsr310.JavaTimeModule;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;
import software.amazon.awssdk.auth.credentials.AwsBasicCredentials;
import software.amazon.awssdk.auth.credentials.StaticCredentialsProvider;
import software.amazon.awssdk.regions.Region;
import software.amazon.awssdk.services.s3.S3Client;
import software.amazon.awssdk.services.s3.S3Configuration;
import software.amazon.awssdk.services.s3.presigner.S3Presigner;
import software.amazon.awssdk.services.sqs.SqsClient;

import java.net.URI;

@Configuration
public class InfraConfig {

    private final AppProperties props;

    public InfraConfig(AppProperties props) {
        this.props = props;
    }

    @Bean
    public ObjectMapper objectMapper() {
        ObjectMapper mapper = new ObjectMapper();
        mapper.registerModule(new JavaTimeModule());
        mapper.setPropertyNamingStrategy(PropertyNamingStrategies.SNAKE_CASE);
        return mapper;
    }

    @Bean
    public S3Client s3Client() {
        var builder = S3Client.builder()
                .region(Region.of(props.getS3().getRegion()))
                .credentialsProvider(creds(props.getS3().getAccessKey(), props.getS3().getSecretKey()))
                .serviceConfiguration(S3Configuration.builder()
                        .pathStyleAccessEnabled(props.getS3().isForcePathStyle())
                        .build());
        if (props.getS3().getEndpoint() != null && !props.getS3().getEndpoint().isBlank()) {
            builder.endpointOverride(URI.create(props.getS3().getEndpoint()));
        }
        return builder.build();
    }

    /**
     * Always signs against the <b>internal</b> {@code S3_ENDPOINT} — matching Rust's
     * {@code ObjectStorage}. GET URLs may be rewritten after signing in MediaService.
     */
    @Bean
    public S3Presigner s3Presigner() {
        var builder = S3Presigner.builder()
                .region(Region.of(props.getS3().getRegion()))
                .credentialsProvider(creds(props.getS3().getAccessKey(), props.getS3().getSecretKey()))
                .serviceConfiguration(S3Configuration.builder()
                        .pathStyleAccessEnabled(props.getS3().isForcePathStyle())
                        .build());
        // Prefer internal endpoint for signing. If only a public HTTPS URL is set
        // (Unraid-style S3_ENDPOINT), still use it — path-style MinIO via tunnel.
        String endpoint = props.getS3().getEndpoint();
        if (endpoint != null && !endpoint.isBlank()) {
            builder.endpointOverride(URI.create(endpoint));
        }
        return builder.build();
    }

    @Bean
    public SqsClient sqsClient() {
        var builder = SqsClient.builder()
                .region(Region.of(props.getQueue().getRegion()))
                .credentialsProvider(creds(props.getQueue().getAccessKey(), props.getQueue().getSecretKey()));
        if (props.getQueue().getEndpoint() != null && !props.getQueue().getEndpoint().isBlank()) {
            builder.endpointOverride(URI.create(props.getQueue().getEndpoint()));
        }
        return builder.build();
    }

    private static StaticCredentialsProvider creds(String access, String secret) {
        String a = access == null || access.isBlank() ? "x" : access;
        String s = secret == null || secret.isBlank() ? "x" : secret;
        return StaticCredentialsProvider.create(AwsBasicCredentials.create(a, s));
    }
}
