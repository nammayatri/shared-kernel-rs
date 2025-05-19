/*  Copyright 2022-23, Juspay India Pvt Ltd
    This program is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License
    as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This program
    is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY
    or FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more details. You should have received a copy of
    the GNU Affero General Public License along with this program. If not, see <https://www.gnu.org/licenses/>.
*/

use crate::call_external_api;
use crate::tools::prometheus::CALL_EXTERNAL_API;
use actix_web::{
    http::{header::ContentType, StatusCode},
    HttpResponse, ResponseError,
};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Client, Method, Response, Url};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::{convert, fmt::Debug};
use tracing::{error, info};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorBody {
    error_message: String,
    pub error_code: String,
}

#[macros::add_error]
pub enum CallAPIError {
    InternalError(String),
    InvalidRequest(String),
    ExternalAPICallError(String),
    SerializationError(String),
    DeserializationError(String),
}

impl CallAPIError {
    fn error_message(&self) -> ErrorBody {
        ErrorBody {
            error_message: self.message(),
            error_code: self.code(),
        }
    }

    pub fn message(&self) -> String {
        match self {
            CallAPIError::InternalError(err) => err.to_string(),
            CallAPIError::InvalidRequest(err) => err.to_string(),
            _ => "Some Error Occured".to_string(),
        }
    }

    fn code(&self) -> String {
        match self {
            CallAPIError::InternalError(_) => "INTERNAL_ERROR",
            CallAPIError::InvalidRequest(_) => "INVALID_REQUEST",
            CallAPIError::ExternalAPICallError(_) => "EXTERNAL_API_CALL_ERROR",
            CallAPIError::SerializationError(_) => "SERIALIZATION_ERROR",
            CallAPIError::DeserializationError(_) => "DESERIALIZATION_ERROR",
        }
        .to_string()
    }
}

impl ResponseError for CallAPIError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .json(self.error_message())
    }

    fn status_code(&self) -> StatusCode {
        match self {
            CallAPIError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            CallAPIError::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            CallAPIError::ExternalAPICallError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            CallAPIError::SerializationError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            CallAPIError::DeserializationError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum Protocol {
    Http1,
    Http2,
}

/// Sends an asynchronous API request to the specified URL.
///
/// This function constructs and sends an HTTP request using the given method, URL, headers, and body.
/// It handles both successful and failed responses, and returns deserialized data in case of success or an `CallAPIError` in case of failure.
///
/// # Arguments
///
/// * `method` - The HTTP method (e.g., GET, POST) for the request.
/// * `url` - A reference to the target URL for the request.
/// * `headers` - A vector containing tuples of header key-value pairs.
/// * `body` - An optional request body. If provided, it will be serialized to JSON.
///
/// # Returns
///
/// * `Ok(T)` if the request succeeds and the response can be deserialized into type `T`.
/// * `Err(CallAPIError)` if there's an error with the request, or if the response status indicates an error.
///
/// # Type Parameters
///
/// * `T`: The expected return type that the response should be deserialized into. Must implement `DeserializeOwned`.
/// * `U`: The type of the request body. Should be serializable into JSON. Must implement `Serialize` and `Debug`.
///
/// # Example
///
/// ```rust
/// let method = Method::GET;
/// let url = Url::parse("https://api.example.com/data").unwrap();
/// let headers = vec![("Authorization", "Bearer TOKEN123")];
///
/// let response: Result<MyResponseType, CallAPIError> = call_api(method, &url, headers, None).await;
/// match response {
///     Ok(data) => println!("Received data: {:?}", data),
///     Err(err) => eprintln!("API error: {}", err),
/// }
/// ```
pub async fn call_api<T, U>(
    protocol: Protocol,
    method: Method,
    url: &Url,
    headers: Vec<(&str, &str)>,
    body: Option<U>,
    service: Option<&str>,
) -> Result<T, CallAPIError>
where
    T: DeserializeOwned + 'static,
    U: Serialize + Debug,
{
    let start_time = std::time::Instant::now();

    let client = match protocol {
        Protocol::Http1 => Ok(Client::new()),
        Protocol::Http2 => Client::builder()
            .http2_prior_knowledge()
            .build()
            .map_err(|err| {
                CallAPIError::InternalError(format!("Http2 client builder error : {err}"))
            }),
    }?;

    // let client = Client::builder().http2_prior_knowledge().build().unwrap();

    let mut header_map = HeaderMap::new();

    for (header_key, header_value) in headers {
        let header_name = HeaderName::from_str(header_key).map_err(|_| {
            CallAPIError::InvalidRequest(format!("Invalid Header Name : {header_key}"))
        })?;
        let header_value = HeaderValue::from_str(header_value).map_err(|_| {
            CallAPIError::InvalidRequest(format!("Invalid Header Value : {header_value}"))
        })?;

        header_map.insert(header_name, header_value);
    }

    let mut request = client
        .request(method.to_owned(), url.to_owned())
        .headers(header_map.to_owned());

    if let Some(body) = &body {
        let body = serde_json::to_string(body)
            .map_err(|err| CallAPIError::SerializationError(err.to_string()))?;
        request = request.body(body);
    }

    let resp = request.send().await;

    let url_str = format!(
        "{}://{}:{}",
        url.scheme(),
        url.host_str().unwrap_or(""),
        url.port().unwrap_or(80)
    );

    let status = match resp.as_ref() {
        Ok(resp) => resp.status().as_str().to_string(),
        Err(err) => err
            .status()
            .map(|status| status.to_string())
            .unwrap_or("UNKNOWN".to_string()),
    };

    call_external_api!(
        method.as_str(),
        url_str.as_str(),
        service.unwrap_or(url.path()),
        status.as_str(),
        start_time
    );

    match resp {
        Ok(resp) => {
            if resp.status().is_success() {
                info!(tag = "[OUTGOING API]", request_method = %method, request_body = format!("{:?}", body), request_url = %url_str, request_headers = format!("{:?}", header_map), response = format!("{:?}", resp), latency = format!("{:?}ms", start_time.elapsed().as_millis()));

                // If T is (), we don't need to deserialize, just return Ok(())
                if std::any::TypeId::of::<T>() == std::any::TypeId::of::<()>() {
                    Ok(unsafe { std::mem::zeroed() })
                } else {
                    Ok(resp
                        .json::<T>()
                        .await
                        .map_err(|err| CallAPIError::DeserializationError(err.to_string()))?)
                }
            } else {
                let resp_status = resp.status().to_string();
                let resp_body = resp.text().await;
                error!(tag = "[OUTGOING API - ERROR]", request_method = %method, request_body = format!("{:?}", body), request_url = %url_str, request_headers = format!("{:?}", header_map), response_status = format!("{:?}", resp_status), response_body = format!("{:?}", resp_body), latency = format!("{:?}ms", start_time.elapsed().as_millis()));
                Err(CallAPIError::ExternalAPICallError(resp_status))
            }
        }
        Err(err) => {
            error!(tag = "[OUTGOING API - ERROR]", request_method = %method, request_body = format!("{:?}", body), request_url = %url_str, request_headers = format!("{:?}", header_map), error = format!("{:?}", err), latency = format!("{:?}ms", start_time.elapsed().as_millis()));
            Err(CallAPIError::ExternalAPICallError(err.to_string()))
        }
    }
}

/// Makes an asynchronous API call, handling errors through a custom error handler.
///
/// This function sends a request to the provided URL using the specified HTTP method, headers, and body.
/// If the request fails, or if the response indicates an error status, it uses the provided error handler
/// to convert the response into an `CallAPIError`.
///
/// # Arguments
///
/// * `method` - The HTTP method to use for the request.
/// * `url` - A reference to the target URL.
/// * `headers` - A vector of header key-value pairs to include in the request.
/// * `body` - An optional request body. If provided, it will be serialized to JSON.
/// * `error_handler` - A boxed function that takes a `Response` and returns an `CallAPIError`.
///                     This is used to convert non-successful responses into appropriate errors.
///
/// # Returns
///
/// * `Ok(T)` if the API call succeeds and the response can be deserialized into type `T`.
/// * `Err(CallAPIError)` if there's any error during the API call, serialization, deserialization,
///                   or if the response status indicates an error.
///
/// # Type Parameters
///
/// * `T`: The type to deserialize the response into. Must implement `DeserializeOwned`.
/// * `U`: The type of the request body. Must implement `Serialize` and `Debug`.
///
/// # Example
///
/// ```rust
/// let method = Method::GET;
/// let url = Url::parse("https://api.example.com/data").unwrap();
/// let headers = vec![("Authorization", "Bearer TOKEN123")];
///
/// async fn error_handler(resp: Response) -> CallAPIError {
///     // Convert the response into an appropriate error here...
/// }
///
/// match call_api_unwrapping_error::<MyResponseType, _>(method, &url, headers, None, Box::new(error_handler)).await {
///     Ok(data) => println!("Received data: {:?}", data),
///     Err(err) => eprintln!("API call error: {}", err),
/// }
/// ```
pub async fn call_api_unwrapping_error<T, U, E>(
    protocol: Protocol,
    method: Method,
    url: &Url,
    headers: Vec<(&str, &str)>,
    body: Option<U>,
    service: Option<&str>,
    error_handler: Box<dyn Fn(Response) -> E>,
) -> Result<T, E>
where
    T: DeserializeOwned + 'static,
    U: Serialize + Debug,
    E: ResponseError + convert::From<CallAPIError>,
{
    let start_time = std::time::Instant::now();

    let client = match protocol {
        Protocol::Http1 => Ok(Client::new()),
        Protocol::Http2 => Client::builder()
            .http2_prior_knowledge()
            .build()
            .map_err(|err| {
                CallAPIError::InternalError(format!("Http2 client builder error : {err}"))
            }),
    }?;

    let mut header_map = HeaderMap::new();

    for (header_key, header_value) in headers {
        let header_name = HeaderName::from_str(header_key).map_err(|_| {
            CallAPIError::InvalidRequest(format!("Invalid Header Name : {header_key}"))
        })?;
        let header_value = HeaderValue::from_str(header_value).map_err(|_| {
            CallAPIError::InvalidRequest(format!("Invalid Header Value : {header_value}"))
        })?;

        header_map.insert(header_name, header_value);
    }

    let mut request = client
        .request(method.to_owned(), url.to_owned())
        .headers(header_map.to_owned());

    if let Some(body) = &body {
        let body = serde_json::to_string(body)
            .map_err(|err| CallAPIError::SerializationError(err.to_string()))?;
        request = request.body(body);
    }

    let resp = request.send().await;

    let url_str = format!(
        "{}://{}:{}",
        url.scheme(),
        url.host_str().unwrap_or(""),
        url.port().unwrap_or(80)
    );

    let status = match resp.as_ref() {
        Ok(resp) => resp.status().as_str().to_string(),
        Err(err) => err
            .status()
            .map(|status| status.to_string())
            .unwrap_or("UNKNOWN".to_string()),
    };

    call_external_api!(
        method.as_str(),
        url_str.as_str(),
        service.unwrap_or(url.path()),
        status.as_str(),
        start_time
    );

    match resp {
        Ok(resp) => {
            if resp.status().is_success() {
                info!(tag = "[OUTGOING API]", request_method = %method, request_body = format!("{:?}", body), request_url = %url_str, request_headers = format!("{:?}", header_map), response = format!("{:?}", resp), latency = format!("{:?}ms", start_time.elapsed().as_millis()));

                // If T is (), we don't need to deserialize, just return Ok(())
                if std::any::TypeId::of::<T>() == std::any::TypeId::of::<()>() {
                    Ok(unsafe { std::mem::zeroed() })
                } else {
                    Ok(resp
                        .json::<T>()
                        .await
                        .map_err(|err| CallAPIError::DeserializationError(err.to_string()))?)
                }
            } else {
                error!(tag = "[OUTGOING API - ERROR]", request_method = %method, request_body = format!("{:?}", body), request_url = %url_str, request_headers = format!("{:?}", header_map), error = format!("{:?}", resp), latency = format!("{:?}ms", start_time.elapsed().as_millis()));
                Err(error_handler(resp))
            }
        }
        Err(err) => {
            error!(tag = "[OUTGOING API - ERROR]", request_method = %method, request_body = format!("{:?}", body), request_url = %url_str, request_headers = format!("{:?}", header_map), error = format!("{:?}", err), latency = format!("{:?}ms", start_time.elapsed().as_millis()));
            Err(CallAPIError::ExternalAPICallError(err.to_string()).into())
        }
    }
}
