use error_stack::Result;
use futures::future::join_all;
use google_cloud_storage::client::{Client, ClientConfig};
use google_cloud_storage::http::objects::download::Range;
use google_cloud_storage::http::objects::get::GetObjectRequest;
use google_cloud_storage::http::objects::list::ListObjectsRequest;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GCSError {
    #[error("Failed to get object from GCS: {0}")]
    GetObjectError(String),

    #[error("Failed to read object data: {0}")]
    ReadDataError(String),

    #[error("Invalid GCS URL format: {0}")]
    InvalidUrlFormat(String),

    #[error("Failed to list objects in GCS: {0}")]
    ListObjectsError(String),

    #[error("Failed to create GCS client: {0}")]
    ClientCreationError(String),
}

/// GCS client
pub struct GCSClient {
    client: Client,
}

impl GCSClient {
    /// Create a new GCS client
    ///
    /// This method will automatically resolve credentials from:
    /// 1. Environment variable GOOGLE_APPLICATION_CREDENTIALS (path to service account JSON)
    /// 2. Application Default Credentials (ADC) when running on GCP
    /// 3. gcloud CLI credentials
    pub async fn new() -> Result<Self, GCSError> {
        let config = ClientConfig::default()
            .with_auth()
            .await
            .map_err(|err| GCSError::ClientCreationError(err.to_string()))?;
        let client = Client::new(config);

        Ok(Self { client })
    }

    /// Fetch an object from GCS by its path
    ///
    /// # Arguments
    ///
    /// * `bucket` - The name of the GCS bucket
    /// * `key` - The key (path) of the object in the bucket
    ///
    /// # Returns
    ///
    /// A Result containing the object data as bytes if successful
    pub async fn fetch_object_gcs(&self, bucket: &str, key: &str) -> Result<Vec<u8>, GCSError> {
        let response = self
            .client
            .download_object(
                &GetObjectRequest {
                    bucket: bucket.to_string(),
                    object: key.to_string(),
                    ..Default::default()
                },
                &Range::default(),
            )
            .await
            .map_err(|err| {
                GCSError::GetObjectError(format!(
                    "Failed to get object from bucket: {}, key: {}: {}",
                    bucket, key, err
                ))
            })?;

        Ok(response)
    }

    /// List objects in a GCS directory
    ///
    /// # Arguments
    ///
    /// * `bucket` - The name of the GCS bucket
    /// * `prefix` - The prefix (directory path) to list objects from
    ///
    /// # Returns
    ///
    /// A Result containing a vector of object keys if successful
    pub async fn list_objects_gcs(
        &self,
        bucket: &str,
        prefix: &str,
    ) -> Result<Vec<String>, GCSError> {
        let mut objects = Vec::new();
        let mut page_token = None;

        loop {
            let response = self
                .client
                .list_objects(&ListObjectsRequest {
                    bucket: bucket.to_string(),
                    prefix: Some(prefix.to_string()),
                    page_token: page_token.clone(),
                    ..Default::default()
                })
                .await
                .map_err(|err| {
                    GCSError::ListObjectsError(format!(
                        "Failed to list objects in bucket: {}, prefix: {}: {}",
                        bucket, prefix, err
                    ))
                })?;

            if let Some(items) = response.items {
                for item in items {
                    objects.push(item.name);
                }
            }

            // Check if there are more objects to fetch
            if let Some(next_page_token) = response.next_page_token {
                page_token = Some(next_page_token);
            } else {
                break;
            }
        }

        Ok(objects)
    }
}

pub async fn get_file_from_gcs(gcs_bucket: &str, gcs_key: &str) -> Result<Vec<u8>, GCSError> {
    // Fetch the object from GCS
    let gcs_client = GCSClient::new().await?;
    let data = gcs_client.fetch_object_gcs(gcs_bucket, gcs_key).await?;

    Ok(data)
}

pub async fn get_files_in_directory_from_gcs(
    gcs_bucket: &str,
    gcs_prefix: &str,
) -> Result<HashMap<String, Vec<u8>>, GCSError> {
    // Get the GCS client
    let gcs_client = GCSClient::new().await?;

    // List all objects in the directory
    let objects = gcs_client.list_objects_gcs(gcs_bucket, gcs_prefix).await?;

    // Download each object
    let mut all_tasks = Vec::new();

    for key in objects {
        // Skip if the key is the prefix itself (directory marker)
        if key == gcs_prefix {
            continue;
        }

        let task = async move {
            let data = get_file_from_gcs(gcs_bucket, &key).await?;
            Ok((key, data))
        };

        all_tasks.push(Box::pin(task));
    }

    let results = join_all(all_tasks)
        .await
        .into_iter()
        .collect::<Result<Vec<(String, Vec<u8>)>, GCSError>>()?;

    let mut files = HashMap::new();

    for (key, data) in results {
        files.insert(key, data);
    }

    Ok(files)
}
