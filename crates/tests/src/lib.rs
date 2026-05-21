/*  Copyright 2022-23, Juspay India Pvt Ltd
    This program is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License
    as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This program
    is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY
    or FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more details. You should have received a copy of
    the GNU Affero General Public License along with this program. If not, see <https://www.gnu.org/licenses/>.
*/
#[test]
fn text_impl_getter_macro() {
    #[macros::impl_getter]
    pub struct Example(pub String);
    // cargo expand --package tests --lib
    let example = Example(String::from("Example"));
    assert_eq!("Example", example.inner());
}

#[tokio::test]
async fn test_download_file() {
    use shared::tools::aws::get_file_from_s3;
    use std::env;

    let _ = env::set_var("AWS_ACCESS_KEY_ID", "...");
    let _ = env::set_var("AWS_SECRET_ACCESS_KEY", "...");
    let _ = env::set_var("AWS_SESSION_TOKEN", "...");

    // Test parameters - replace with your actual bucket and key
    let bucket = "route-geojson";
    let key = "AC23A-D.geojson";

    // Download the file
    let data = get_file_from_s3(bucket, key).await.unwrap();

    let data = String::from_utf8(data).unwrap();

    println!(
        "Successfully fetched file from S3: {}/{} => {}",
        bucket, key, data
    );
}

#[tokio::test]
async fn test_download_files_from_directory() {
    use shared::tools::aws::get_files_in_directory_from_s3;
    use std::env;

    let _ = env::set_var("AWS_ACCESS_KEY_ID", "...");
    let _ = env::set_var("AWS_SECRET_ACCESS_KEY", "...");
    let _ = env::set_var("AWS_SESSION_TOKEN", "...");

    // Test parameters - replace with your actual bucket and key
    let bucket = "route-geojson";
    let prefix = "";

    // Download the file
    let data = get_files_in_directory_from_s3(bucket, prefix)
        .await
        .unwrap();

    for (key, data) in data {
        let data = String::from_utf8(data).unwrap();
        println!(
            "Successfully fetched file from S3: {}/{} => {}",
            bucket, key, data
        );
    }
}

#[tokio::test]
#[ignore] // Requires GCS credentials - run with: cargo test -- --ignored
async fn test_list_objects_gcs() {
    use shared::tools::gcs::GCSClient;
    use std::env;

    let bucket = env::var("GCS_TEST_BUCKET")
        .unwrap_or_else(|_| "beckn-image-gcs-bucket".to_string());
    let prefix = "";

    let gcs_client = match GCSClient::new().await {
        Ok(client) => client,
        Err(e) => {
            eprintln!("Failed to create GCS client: {:?}", e);
            eprintln!("\nTo authenticate with GCS, you need to:");
            eprintln!("  1. Set GOOGLE_APPLICATION_CREDENTIALS environment variable to path of service account JSON, OR");
            eprintln!("  2. Run: gcloud auth application-default login");
            eprintln!("  3. Ensure you have proper permissions to access bucket: {}", bucket);
            panic!("GCS authentication failed");
        }
    };
    
    let objects = match gcs_client.list_objects_gcs(&bucket, prefix).await {
        Ok(objs) => objs,
        Err(e) => {
            eprintln!("Failed to list objects: {:?}", e);
            eprintln!("Bucket: {}, Prefix: '{}'", bucket, prefix);
            panic!("Failed to list GCS objects");
        }
    };

    println!("Found {} objects in bucket: {}", objects.len(), bucket);
    for (idx, object) in objects.iter().take(10).enumerate() {
        println!("  {}. {}", idx + 1, object);
    }
    if objects.len() > 10 {
        println!("  ... and {} more objects", objects.len() - 10);
    }
}

#[tokio::test]
#[ignore] // Requires GCS credentials - run with: cargo test -- --ignored
async fn test_download_file_gcs() {
    use shared::tools::gcs::{get_file_from_gcs, GCSClient};
    use std::env;

    let bucket = env::var("GCS_TEST_BUCKET")
        .unwrap_or_else(|_| "beckn-image-gcs-bucket".to_string());
    
    // First, let's list objects to find a file to download
    let gcs_client = match GCSClient::new().await {
        Ok(client) => client,
        Err(e) => {
            eprintln!("Failed to create GCS client: {:?}", e);
            eprintln!("\nTo authenticate with GCS, you need to:");
            eprintln!("  1. Set GOOGLE_APPLICATION_CREDENTIALS environment variable to path of service account JSON, OR");
            eprintln!("  2. Run: gcloud auth application-default login");
            panic!("GCS authentication failed");
        }
    };
    
    let objects = match gcs_client.list_objects_gcs(&bucket, "").await {
        Ok(objs) => objs,
        Err(e) => {
            eprintln!("Failed to list objects: {:?}", e);
            panic!("Failed to list GCS objects");
        }
    };
    
    if objects.is_empty() {
        println!("No objects found in bucket: {}", bucket);
        return;
    }

    // Use the first object as a test
    let key = &objects[0];
    println!("Testing download of: {}/{}", bucket, key);

    // Download the file
    let data = match get_file_from_gcs(&bucket, key).await {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Failed to download file: {:?}", e);
            panic!("Failed to download GCS file");
        }
    };

    println!(
        "Successfully fetched file from GCS: {}/{} => {} bytes",
        bucket, key, data.len()
    );
    
    // Try to display as string if it's text, otherwise show first few bytes
    if let Ok(text) = String::from_utf8(data.clone()) {
        let preview = if text.len() > 200 {
            format!("{}...", &text[..200])
        } else {
            text
        };
        println!("File content preview: {}", preview);
    } else {
        println!("File is binary, size: {} bytes", data.len());
        if !data.is_empty() {
            println!("First 20 bytes: {:?}", &data[..data.len().min(20)]);
        }
    }
}

#[tokio::test]
#[ignore] // Requires GCS credentials - run with: cargo test -- --ignored
async fn test_download_files_from_directory_gcs() {
    use shared::tools::gcs::get_files_in_directory_from_gcs;
    use std::env;

    let bucket = env::var("GCS_TEST_BUCKET")
        .unwrap_or_else(|_| "beckn-image-gcs-bucket".to_string());
    let prefix = ""; // Empty prefix means root of bucket

    println!("Downloading files from GCS bucket: {} with prefix: '{}'", bucket, prefix);

    // Download the files
    let data = match get_files_in_directory_from_gcs(&bucket, prefix).await {
        Ok(files) => files,
        Err(e) => {
            eprintln!("Failed to download files from directory: {:?}", e);
            eprintln!("\nTo authenticate with GCS, you need to:");
            eprintln!("  1. Set GOOGLE_APPLICATION_CREDENTIALS environment variable to path of service account JSON, OR");
            eprintln!("  2. Run: gcloud auth application-default login");
            panic!("GCS authentication failed");
        }
    };

    println!("Successfully fetched {} files from GCS bucket: {}", data.len(), bucket);
    
    for (key, file_data) in data.iter().take(5) {
        println!(
            "  File: {} => {} bytes",
            key, file_data.len()
        );
    }
    
    if data.len() > 5 {
        println!("  ... and {} more files", data.len() - 5);
    }
}

#[tokio::test]
#[ignore] // Requires GCS credentials - run with: cargo test -- --ignored
async fn test_cloud_storage_wrapper_gcs() {
    use shared::tools::cloud_storage::{get_files_in_directory, CloudProvider};
    use std::env;

    let bucket = env::var("GCS_TEST_BUCKET")
        .unwrap_or_else(|_| "beckn-image-gcs-bucket".to_string());
    let prefix = "";

    println!("Testing cloud storage wrapper with GCS provider");

    let files = match get_files_in_directory(CloudProvider::GCS, &bucket, prefix).await {
        Ok(files) => files,
        Err(e) => {
            eprintln!("Failed to get files: {:?}", e);
            panic!("Failed to get files from GCS via wrapper");
        }
    };

    println!("Successfully fetched {} files using cloud storage wrapper (GCS)", files.len());
    
    for (key, file_data) in files.iter().take(5) {
        println!("  File: {} => {} bytes", key, file_data.len());
    }
    
    if files.len() > 5 {
        println!("  ... and {} more files", files.len() - 5);
    }
}

#[tokio::test]
#[ignore] // Requires GCS credentials - run with: cargo test -- --ignored
async fn test_cloud_storage_wrapper_from_str() {
    use shared::tools::cloud_storage::get_files_in_directory_from_str;
    use std::env;

    let bucket = env::var("GCS_TEST_BUCKET")
        .unwrap_or_else(|_| "beckn-image-gcs-bucket".to_string());
    let prefix = "";

    println!("Testing cloud storage wrapper with string provider 'gcs'");

    let files = match get_files_in_directory_from_str("gcs", &bucket, prefix).await {
        Ok(files) => files,
        Err(e) => {
            eprintln!("Failed to get files: {:?}", e);
            panic!("Failed to get files from GCS via string wrapper");
        }
    };

    println!("Successfully fetched {} files using string-based wrapper", files.len());
    
    // Test invalid provider
    let result = get_files_in_directory_from_str("invalid", &bucket, prefix).await;
    assert!(result.is_err(), "Should fail with invalid provider");
    println!("Correctly rejected invalid provider 'invalid'");
}

// ============================================================================
// PolylineIndex (shared::tools::polyline)
// ============================================================================

#[test]
fn polyline_haversine_known_distance() {
    use shared::tools::polyline::haversine_meters;
    // Two points ~447m apart in Chennai
    let a = (13.03627, 80.24666);
    let b = (13.03956, 80.24435);
    let d = haversine_meters(a, b);
    assert!((400.0..500.0).contains(&d), "expected ~447m, got {d}");
}

#[test]
fn polyline_haversine_zero_for_same_point() {
    use shared::tools::polyline::haversine_meters;
    let p = (13.0, 80.0);
    assert_eq!(haversine_meters(p, p), 0.0);
}

#[test]
fn polyline_empty_index_returns_none() {
    use shared::tools::polyline::PolylineIndex;
    let idx = PolylineIndex::build(&[]);
    assert!(idx.is_empty());
    assert!(idx.find_nearest_point(13.0, 80.0).is_none());
    assert!(idx.distance_between((13.0, 80.0), (13.1, 80.1)).is_none());
}

#[test]
fn polyline_nearest_point_on_simple_line() {
    use shared::tools::polyline::PolylineIndex;
    let pts = vec![(13.00, 80.00), (13.01, 80.00), (13.02, 80.00), (13.03, 80.00)];
    let idx = PolylineIndex::build(&pts);
    let (i, d) = idx.find_nearest_point(13.02, 80.00).unwrap();
    assert_eq!(i, 2);
    assert!(d < 1.0, "expected ~0m, got {d}");
}

#[test]
fn polyline_nearest_point_off_line() {
    use shared::tools::polyline::PolylineIndex;
    let pts = vec![(13.00, 80.00), (13.01, 80.00), (13.02, 80.00)];
    let idx = PolylineIndex::build(&pts);
    let (i, d) = idx.find_nearest_point(13.01, 80.001).unwrap();
    assert_eq!(i, 1);
    assert!(d > 50.0 && d < 200.0, "expected ~100m off-line, got {d}");
}

#[test]
fn polyline_cumulative_distance_monotonic() {
    use shared::tools::polyline::PolylineIndex;
    let pts = vec![(13.00, 80.00), (13.01, 80.00), (13.02, 80.00), (13.03, 80.00)];
    let idx = PolylineIndex::build(&pts);
    let d_short = idx.distance_between((13.00, 80.00), (13.01, 80.00)).unwrap();
    let d_long = idx.distance_between((13.00, 80.00), (13.03, 80.00)).unwrap();
    assert!(d_long > d_short);
    assert!(d_long > 3000.0 && d_long < 3500.0, "expected ~3.3km, got {d_long}");
}

#[test]
fn polyline_distance_between_symmetric() {
    use shared::tools::polyline::PolylineIndex;
    let pts = vec![(13.00, 80.00), (13.01, 80.00), (13.02, 80.00)];
    let idx = PolylineIndex::build(&pts);
    let a = idx.distance_between((13.00, 80.00), (13.02, 80.00)).unwrap();
    let b = idx.distance_between((13.02, 80.00), (13.00, 80.00)).unwrap();
    assert!((a - b).abs() < 0.001);
}
