use aws_sdk_s3::client::Client;
use error_stack::Result;
use futures::future::join_all;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AWSError {
    #[error("Failed to get object from S3: {0}")]
    GetObjectError(String),

    #[error("Failed to read object data: {0}")]
    ReadDataError(String),

    #[error("Invalid S3 URL format: {0}")]
    InvalidUrlFormat(String),

    #[error("Failed to list objects in S3: {0}")]
    ListObjectsError(String),

    #[error("Failed to create directory: {0}")]
    CreateDirectoryError(String),
}

/// AWS client
pub struct AWSClient {
    client: Client,
}

impl AWSClient {
    /// Create a new S3 client with the specified region
    ///
    /// This method will automatically resolve credentials from:
    /// 1. Environment variables (AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY)
    /// 2. AWS credentials file (~/.aws/credentials)
    /// 3. IAM instance profile (when running on AWS)
    /// 4. ECS container credentials (when running in ECS)
    /// 5. EKS pod identity (when running in EKS)
    pub async fn new() -> Result<Self, AWSError> {
        // Load configuration from environment, credentials file, and instance profile
        let config = aws_config::from_env().load().await;

        let client = Client::new(&config);

        Ok(Self { client })
    }

    /// Fetch an object from S3 by its path
    ///
    /// # Arguments
    ///
    /// * `bucket` - The name of the S3 bucket
    /// * `key` - The key (path) of the object in the bucket
    ///
    /// # Returns
    ///
    /// A Result containing the object data as bytes if successful
    pub async fn fetch_object_s3(&self, bucket: &str, key: &str) -> Result<Vec<u8>, AWSError> {
        let response = self
            .client
            .get_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .map_err(|_err| {
                AWSError::GetObjectError(format!(
                    "Failed to get object from bucket: {}, key: {}",
                    bucket, key
                ))
            })?;

        let data =
            response.body.collect().await.map_err(|_err| {
                AWSError::ReadDataError("Failed to read object data".to_string())
            })?;

        Ok(data.into_bytes().to_vec())
    }

    /// List objects in an S3 directory
    ///
    /// # Arguments
    ///
    /// * `bucket` - The name of the S3 bucket
    /// * `prefix` - The prefix (directory path) to list objects from
    ///
    /// # Returns
    ///
    /// A Result containing a vector of object keys if successful
    pub async fn list_objects_s3(
        &self,
        bucket: &str,
        prefix: &str,
    ) -> Result<Vec<String>, AWSError> {
        let mut objects = Vec::new();
        let mut continuation_token = None;

        loop {
            let mut list_request = self.client.list_objects_v2().bucket(bucket).prefix(prefix);

            if let Some(token) = continuation_token {
                list_request = list_request.continuation_token(token);
            }

            let response = list_request.send().await.map_err(|_err| {
                AWSError::ListObjectsError(format!(
                    "Failed to list objects in bucket: {}, prefix: {}",
                    bucket, prefix
                ))
            })?;

            if let Some(contents) = response.contents.to_owned() {
                for object in contents {
                    if let Some(key) = object.key() {
                        objects.push(key.to_string());
                    }
                }
            }

            // Check if there are more objects to fetch
            if let Some(next_continuation_token) = response.next_continuation_token() {
                continuation_token = Some(next_continuation_token.to_string());
            } else {
                break;
            }
        }

        Ok(objects)
    }
}

pub async fn get_file_from_s3(s3_bucket: &str, s3_key: &str) -> Result<Vec<u8>, AWSError> {
    // Fetch the object from S3
    let s3_client = AWSClient::new().await?;
    let data = s3_client.fetch_object_s3(s3_bucket, s3_key).await?;

    Ok(data)
}

pub async fn get_files_in_directory_from_s3(
    s3_bucket: &str,
    s3_prefix: &str,
) -> Result<HashMap<String, Vec<u8>>, AWSError> {
    // Get the S3 client
    let s3_client = AWSClient::new().await?;

    // List all objects in the directory
    let objects = s3_client.list_objects_s3(s3_bucket, s3_prefix).await?;

    // Download each object
    let mut all_tasks = Vec::new();

    for key in objects {
        // Skip if the key is the prefix itself (directory marker)
        if key == s3_prefix {
            continue;
        }

        let task = async move {
            let data = get_file_from_s3(s3_bucket, &key).await?;
            Ok((key, data))
        };

        all_tasks.push(Box::pin(task));
    }

    let results = join_all(all_tasks)
        .await
        .into_iter()
        .collect::<Result<Vec<(String, Vec<u8>)>, AWSError>>()?;

    let mut files = HashMap::new();

    for (key, data) in results {
        files.insert(key, data);
    }

    Ok(files)
}
