/*  Copyright 2022-23, Juspay India Pvt Ltd
    This program is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License
    as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This program
    is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY
    or FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more details. You should have received a copy of
    the GNU Affero General Public License along with this program. If not, see <https://www.gnu.org/licenses/>.
*/

use crate::incoming_api;
use crate::tools::prometheus::INCOMING_API;
use actix_http::StatusCode;
use actix_web::{Error, HttpRequest};
use once_cell::sync::Lazy;
use regex::Regex;
use tokio::time::Instant;
use tracing::{error, info};

static UUID_REGEX: Lazy<Option<Regex>> = Lazy::new(|| {
    Regex::new(r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}").ok()
});

/// Get the path from the HTTP request.
///
/// Retrieves the path from the incoming request and replaces any matched info with placeholders.
///
/// # Arguments
/// * `request` - The incoming HTTP request.
///
/// # Returns
/// * `String` - The path string with placeholders for matched info.
pub fn get_path(request: &HttpRequest) -> String {
    let mut path = urlencoding::decode(request.path())
        .map(|s| s.into_owned())
        .unwrap_or_else(|_| request.path().to_owned());

    request
        .match_info()
        .iter()
        .for_each(|(path_name, path_val)| {
            path = path.replace(path_val, &format!(":{path_name}"));
        });

    if let Some(re) = UUID_REGEX.as_ref() {
        path = re.replace_all(&path, ":id").into_owned();
    }

    path
}

/// Get the method from the HTTP request.
///
/// Retrieves the HTTP method (e.g., GET, POST) from the incoming request.
///
/// # Arguments
/// * `request` - The incoming HTTP request.
///
/// # Returns
/// * `String` - The HTTP method as a string.
#[inline]
pub fn get_method(request: &HttpRequest) -> String {
    request.method().to_string()
}

/// Get the headers from the HTTP request.
///
/// Retrieves and formats the headers from the incoming HTTP request.
///
/// # Arguments
/// * `request` - The incoming HTTP request.
///
/// # Returns
/// * `String` - A formatted string representation of the headers.
#[inline]
pub fn get_headers(request: &HttpRequest) -> String {
    format!("{:?}", request.headers())
}

/// Calculate and log metrics from HTTP requests and responses.
///
/// This function calculates metrics such as latency and logs information including
/// error responses, HTTP methods, paths, and headers.
///
/// # Arguments
/// * `err_resp` - Optional reference to an error response.
/// * `resp_status` - The status code of the response.
/// * `req_headers` - A string representation of the request headers.
/// * `req_method` - The HTTP method of the request as a string.
/// * `req_path` - The path of the request as a string.
/// * `time` - The instant at which the request was received.
pub fn calculate_metrics(
    err_resp: Option<&Error>,
    resp_status: StatusCode,
    req_headers: &str,
    req_method: &str,
    req_path: &str,
    time: Instant,
) {
    if let Some(err_resp) = err_resp {
        let err_resp_code = err_resp.to_string();
        error!(tag = "[INCOMING API - ERROR]", request_method = %req_method, request_path = %req_path, request_headers = req_headers, response_code = err_resp_code, response_status = resp_status.as_str(), latency = format!("{:?}ms", time.elapsed().as_millis()));
        incoming_api!(
            req_method,
            req_path,
            resp_status.as_str(),
            err_resp_code.as_str(),
            time
        );
    } else {
        info!(tag = "[INCOMING API]", request_method = %req_method, request_path = %req_path, request_headers = req_headers, response_status = resp_status.as_str(), latency = format!("{:?}ms", time.elapsed().as_millis()));
        incoming_api!(req_method, req_path, resp_status.as_str(), "SUCCESS", time);
    }
}
