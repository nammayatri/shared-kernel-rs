use crate::tools::aws::{get_files_in_directory_from_s3, AWSError};
use crate::tools::gcs::{get_files_in_directory_from_gcs, GCSError};
use error_stack::Result;
use std::collections::HashMap;
use thiserror::Error;

/// Cloud storage provider type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudProvider {
    /// Amazon Web Services S3
    AWS,
    /// Google Cloud Storage
    GCS,
}

impl CloudProvider {
    /// Parse from string (case-insensitive)
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "AWS" | "S3" => Some(CloudProvider::AWS),
            "GCS" | "GCP" | "GOOGLE" => Some(CloudProvider::GCS),
            _ => None,
        }
    }
}

/// Unified error type for cloud storage operations
#[derive(Error, Debug)]
pub enum CloudStorageError {
    #[error("AWS S3 error: {0}")]
    AWS(#[from] AWSError),

    #[error("Google Cloud Storage error: {0}")]
    GCS(#[from] GCSError),

    #[error("Invalid cloud provider: {0}")]
    InvalidProvider(String),
}

/// Get files from a directory in cloud storage (S3 or GCS)
///
/// # Arguments
///
/// * `provider` - The cloud provider to use (AWS or GCS)
/// * `bucket` - The name of the bucket
/// * `prefix` - The prefix (directory path) to list objects from
///
/// # Returns
///
/// A Result containing a HashMap mapping file keys to their contents
///
/// # Example
///
/// ```rust,no_run
/// use shared::tools::cloud_storage::{get_files_in_directory, CloudProvider};
///
/// // Use AWS S3
/// let files = get_files_in_directory(
///     CloudProvider::AWS,
///     "my-bucket",
///     "path/to/directory/"
/// ).await?;
///
/// // Use Google Cloud Storage
/// let files = get_files_in_directory(
///     CloudProvider::GCS,
///     "my-bucket",
///     "path/to/directory/"
/// ).await?;
/// ```
pub async fn get_files_in_directory(
    provider: CloudProvider,
    bucket: &str,
    prefix: &str,
) -> Result<HashMap<String, Vec<u8>>, CloudStorageError> {
    match provider {
        CloudProvider::AWS => {
            let result = get_files_in_directory_from_s3(bucket, prefix).await;
            result.map_err(|err| {
                // Extract the inner AWS error from the error_stack::Report
                // We'll create a new CloudStorageError with the error message
                let error_msg = format!("{}", err);
                error_stack::Report::new(CloudStorageError::AWS(AWSError::GetObjectError(error_msg)))
            })
        }
        CloudProvider::GCS => {
            let result = get_files_in_directory_from_gcs(bucket, prefix).await;
            result.map_err(|err| {
                // Extract the inner GCS error from the error_stack::Report
                // We'll create a new CloudStorageError with the error message
                let error_msg = format!("{}", err);
                error_stack::Report::new(CloudStorageError::GCS(GCSError::GetObjectError(error_msg)))
            })
        }
    }
}

/// Get files from a directory in cloud storage using a string provider identifier
///
/// # Arguments
///
/// * `provider_str` - The cloud provider as a string ("aws", "s3", "gcs", "gcp", "google")
/// * `bucket` - The name of the bucket
/// * `prefix` - The prefix (directory path) to list objects from
///
/// # Returns
///
/// A Result containing a HashMap mapping file keys to their contents
///
/// # Example
///
/// ```rust,no_run
/// use shared::tools::cloud_storage::get_files_in_directory_from_str;
///
/// // Use AWS S3
/// let files = get_files_in_directory_from_str(
///     "aws",
///     "my-bucket",
///     "path/to/directory/"
/// ).await?;
///
/// // Use Google Cloud Storage
/// let files = get_files_in_directory_from_str(
///     "gcs",
///     "my-bucket",
///     "path/to/directory/"
/// ).await?;
/// ```
pub async fn get_files_in_directory_from_str(
    provider_str: &str,
    bucket: &str,
    prefix: &str,
) -> Result<HashMap<String, Vec<u8>>, CloudStorageError> {
    let provider = CloudProvider::from_str(provider_str)
        .ok_or_else(|| {
            error_stack::Report::new(CloudStorageError::InvalidProvider(provider_str.to_string()))
        })?;

    get_files_in_directory(provider, bucket, prefix).await
}
