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
