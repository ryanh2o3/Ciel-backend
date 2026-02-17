use anyhow::{anyhow, Result};
use aws_config::meta::region::RegionProviderChain;
use aws_config::BehaviorVersion;
use aws_config::Region;
use aws_sdk_sqs::config::Credentials;
use aws_sdk_sqs::error::SdkError;
use aws_sdk_sqs::Client;
use serde_json;
use tracing::{debug, warn};

use crate::config::AppConfig;
use crate::jobs::media_processor::MediaJob;

#[derive(Clone)]
pub struct QueueClient {
    client: Client,
    queue_name: String,
    queue_url: String,
}

#[derive(Debug)]
pub struct ReceivedJob {
    pub job: MediaJob,
    pub receipt_handle: String,
}

impl QueueClient {
    pub async fn new(config: &AppConfig) -> Result<Self> {
        let region = Region::new(config.queue_region.clone());

        let mut sqs_builder = aws_sdk_sqs::config::Builder::new()
            .region(region)
            .endpoint_url(config.queue_endpoint.clone());

        // Use explicit SQS credentials if provided, otherwise fall back to default chain
        if !config.sqs_access_key.is_empty() {
            sqs_builder = sqs_builder.credentials_provider(Credentials::new(
                &config.sqs_access_key,
                &config.sqs_secret_key,
                None,
                None,
                "env",
            ));
        } else {
            let region_provider = RegionProviderChain::first_try(Region::new(config.queue_region.clone()));
            let shared_config = aws_config::defaults(BehaviorVersion::latest())
                .region(region_provider)
                .load()
                .await;
            if let Some(provider) = shared_config.credentials_provider() {
                sqs_builder = sqs_builder.credentials_provider(provider);
            }
        }

        let sqs_config = sqs_builder.build();

        let client = Client::from_conf(sqs_config);
        let queue_url = match client
            .get_queue_url()
            .queue_name(&config.queue_name)
            .send()
            .await
        {
            Ok(response) => response
                .queue_url()
                .ok_or_else(|| anyhow!("missing queue url"))?
                .to_string(),
            Err(SdkError::ServiceError(service_err))
                if service_err.err().is_queue_does_not_exist() =>
            {
                let created = client
                    .create_queue()
                    .queue_name(&config.queue_name)
                    .send()
                    .await?;
                created
                    .queue_url()
                    .ok_or_else(|| anyhow!("missing queue url"))?
                    .to_string()
            }
            Err(err) => return Err(anyhow!(err)),
        };

        Ok(Self {
            client,
            queue_name: config.queue_name.clone(),
            queue_url,
        })
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    pub fn queue_name(&self) -> &str {
        &self.queue_name
    }

    pub async fn enqueue_media_job(&self, job: &MediaJob) -> Result<()> {
        let body = serde_json::to_string(job)?;
        self.client
            .send_message()
            .queue_url(&self.queue_url)
            .message_body(body)
            .send()
            .await?;

        Ok(())
    }

    pub async fn receive_media_job(&self, wait_time_seconds: i32) -> Result<Option<ReceivedJob>> {
        let response = self
            .client
            .receive_message()
            .queue_url(&self.queue_url)
            .max_number_of_messages(1)
            .wait_time_seconds(wait_time_seconds)
            .send()
            .await?;

        let message = match response.messages().first() {
            Some(message) => message,
            None => return Ok(None),
        };

        let receipt_handle = match message.receipt_handle() {
            Some(handle) => handle.to_string(),
            None => {
                warn!("queue message missing receipt handle");
                return Ok(None);
            }
        };

        let body = match message.body() {
            Some(body) => body,
            None => {
                warn!("queue message missing body, deleting");
                let _ = self.delete_message(&receipt_handle).await;
                return Ok(None);
            }
        };

        let job: MediaJob = match serde_json::from_str(body) {
            Ok(job) => job,
            Err(err) => {
                warn!(error = ?err, "failed to parse queue message body");
                let _ = self.delete_message(&receipt_handle).await;
                return Ok(None);
            }
        };

        debug!(upload_id = %job.upload_id, "received media job");
        Ok(Some(ReceivedJob { job, receipt_handle }))
    }

    pub async fn delete_message(&self, receipt_handle: &str) -> Result<()> {
        self.client
            .delete_message()
            .queue_url(&self.queue_url)
            .receipt_handle(receipt_handle)
            .send()
            .await?;

        Ok(())
    }
}

