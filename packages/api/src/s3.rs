//! MinIO / S3 client construction.

use aws_config::BehaviorVersion;
use aws_credential_types::Credentials;
use aws_sdk_s3::{config::Region, Client};

use crate::config::AppConfig;

/// Build an `aws_sdk_s3::Client` configured for MinIO (path-style + static creds).
pub async fn build_client(cfg: &AppConfig) -> Client {
    let creds = Credentials::new(&cfg.s3_access_key, &cfg.s3_secret_key, None, None, "vonk-env");

    let shared = aws_config::defaults(BehaviorVersion::latest())
        .region(Region::new(cfg.s3_region.clone()))
        .credentials_provider(creds)
        .endpoint_url(&cfg.s3_endpoint)
        .load()
        .await;

    let s3_conf = aws_sdk_s3::config::Builder::from(&shared)
        .force_path_style(cfg.s3_force_path_style)
        .build();

    Client::from_conf(s3_conf)
}
