use anyhow::Result;
use aws_config::meta::region::RegionProviderChain;
use aws_config::BehaviorVersion;
use aws_config::Region;
use aws_sdk_s3::Client;

use crate::config::AppConfig;

#[derive(Clone)]
pub struct ObjectStorage {
    client: Client,
    bucket: String,
}

impl ObjectStorage {
    pub async fn new(config: &AppConfig) -> Result<Self> {
        let region_provider = RegionProviderChain::first_try(Region::new(config.s3_region.clone()));
        let shared_config = aws_config::defaults(BehaviorVersion::latest())
            .region(region_provider)
            .load()
            .await;

        let mut s3_builder = aws_sdk_s3::config::Builder::from(&shared_config)
            .region(shared_config.region().cloned())
            .endpoint_url(config.s3_endpoint.clone())
            .force_path_style(true);
        if let Some(provider) = shared_config.credentials_provider() {
            s3_builder = s3_builder.credentials_provider(provider);
        }
        let s3_config = s3_builder.build();

        let client = Client::from_conf(s3_config);

        Ok(Self {
            client,
            bucket: config.s3_bucket.clone(),
        })
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    pub fn bucket(&self) -> &str {
        &self.bucket
    }
}

