use crate::tools::aws::{get_files_in_directory_from_s3, AWSError};
use crate::tools::gcs::{get_files_in_directory_from_gcs, GCSError};
use error_stack::Result;
use std::collections::HashMap;
use std::str::FromStr;
use thiserror::Error;

/// Cloud storage provider type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudProvider {
    /// Amazon Web Services S3
    AWS,
    /// Google Cloud Storage
    GCS,
}

impl FromStr for CloudProvider {
    type Err = String;

    /// Parse from string (case-insensitive)
    ///
    /// Supported values:
    /// - AWS: "aws", "s3"
    /// - GCS: "gcs", "gcp", "google"
    #[inline]
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "AWS" | "S3" => Ok(CloudProvider::AWS),
            "GCS" | "GCP" | "GOOGLE" => Ok(CloudProvider::GCS),
            _ => Err(format!(
                "Invalid cloud provider: '{}'. Supported values: aws, s3, gcs, gcp, google",
                s
            )),
        }
    }
}

impl CloudProvider {
    /// Parse from string (case-insensitive) - returns Option for backward compatibility
    #[deprecated(note = "Use std::str::FromStr::from_str or parse() instead")]
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        s.parse().ok()
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
/// use shared::tools::cloud_storage::{get_files_in_directory, CloudProvider, CloudStorageError};
///
/// # async fn example() -> Result<(), CloudStorageError> {
/// // Use AWS S3
/// let aws_files = get_files_in_directory(
///     CloudProvider::AWS,
///     "my-bucket",
///     "path/to/directory/"
/// ).await?;
///
/// // Use Google Cloud Storage
/// let gcs_files = get_files_in_directory(
///     CloudProvider::GCS,
///     "my-bucket",
///     "path/to/directory/"
/// ).await?;
///
/// // Do something with the files...
/// # Ok(())
/// # }
/// ```
pub async fn get_files_in_directory(
    provider: CloudProvider,
    bucket: &str,
    prefix: &str,
) -> Result<HashMap<String, Vec<u8>>, CloudStorageError> {
    match provider {
        CloudProvider::AWS => {
            get_files_in_directory_from_s3(bucket, prefix)
                .await
                .map_err(|err| {
                    // Try to extract the original AWS error from the error_stack::Report
                    // If we can't extract it, create a generic error with the message
                    if let Some(aws_err) = err.downcast_ref::<AWSError>() {
                        // Clone the error - we'll need to reconstruct it since AWSError may not implement Clone
                        // For now, preserve the error message
                        error_stack::Report::new(CloudStorageError::AWS(AWSError::GetObjectError(
                            format!("{}", aws_err),
                        )))
                    } else {
                        // Fallback: use the error message
                        error_stack::Report::new(CloudStorageError::AWS(AWSError::GetObjectError(
                            format!("{}", err),
                        )))
                    }
                })
        }
        CloudProvider::GCS => {
            get_files_in_directory_from_gcs(bucket, prefix)
                .await
                .map_err(|err| {
                    // Try to extract the original GCS error from the error_stack::Report
                    // If we can't extract it, create a generic error with the message
                    if let Some(gcs_err) = err.downcast_ref::<GCSError>() {
                        error_stack::Report::new(CloudStorageError::GCS(GCSError::GetObjectError(
                            format!("{}", gcs_err),
                        )))
                    } else {
                        // Fallback: use the error message
                        error_stack::Report::new(CloudStorageError::GCS(GCSError::GetObjectError(
                            format!("{}", err),
                        )))
                    }
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
/// use shared::tools::cloud_storage::{get_files_in_directory_from_str, CloudStorageError};
///
/// # async fn example() -> Result<(), CloudStorageError> {
/// // Use AWS S3
/// let aws_files = get_files_in_directory_from_str(
///     "aws",
///     "my-bucket",
///     "path/to/directory/"
/// ).await?;
///
/// // Use Google Cloud Storage
/// let gcs_files = get_files_in_directory_from_str(
///     "gcs",
///     "my-bucket",
///     "path/to/directory/"
/// ).await?;
///
/// // Do something with the files...
/// # Ok(())
/// # }
/// ```
pub async fn get_files_in_directory_from_str(
    provider_str: &str,
    bucket: &str,
    prefix: &str,
) -> Result<HashMap<String, Vec<u8>>, CloudStorageError> {
    let provider = provider_str.parse::<CloudProvider>().map_err(|e| {
        error_stack::Report::new(CloudStorageError::InvalidProvider(provider_str.to_string()))
            .attach_printable(e)
    })?;

    get_files_in_directory(provider, bucket, prefix).await
}
