package com.ciel.config;

import jakarta.annotation.PostConstruct;
import org.springframework.stereotype.Component;
import software.amazon.awssdk.services.sqs.SqsClient;
import software.amazon.awssdk.services.sqs.model.QueueAttributeName;

@Component
public class SqsQueueUrl {

    private final SqsClient sqsClient;
    private final AppProperties props;
    private String queueUrl;

    public SqsQueueUrl(SqsClient sqsClient, AppProperties props) {
        this.sqsClient = sqsClient;
        this.props = props;
    }

    @PostConstruct
    void resolve() {
        String name = props.getQueue().getName();
        try {
            queueUrl = sqsClient.getQueueUrl(r -> r.queueName(name)).queueUrl();
        } catch (Exception notFound) {
            queueUrl = sqsClient
                    .createQueue(r -> r.queueName(name)
                            .attributes(java.util.Map.of(
                                    QueueAttributeName.VISIBILITY_TIMEOUT, "300")))
                    .queueUrl();
        }
    }

    public String get() {
        return queueUrl;
    }
}
