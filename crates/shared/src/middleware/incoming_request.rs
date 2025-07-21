/*  Copyright 2022-23, Juspay India Pvt Ltd
    This program is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License
    as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This program
    is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY
    or FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more details. You should have received a copy of
    the GNU Affero General Public License along with this program. If not, see <https://www.gnu.org/licenses/>.
*/

use crate::middleware::utils::{calculate_metrics, get_headers, get_method, get_path};
use actix::fut::{ready, Ready};
use actix_web::{
    body::BoxBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error,
};
use futures::future::LocalBoxFuture;
use tokio::time::Instant;

/// Responsible for collecting metrics from incoming requests and their responses.
///
/// `IncomingRequestMetrics` acts as a middleware, capturing essential information and
/// metrics from incoming service requests and the corresponding responses or errors.
/// The collected metrics can include headers, paths, methods, and the duration of request handling.
pub struct IncomingRequestMetrics;

impl<S> Transform<S, ServiceRequest> for IncomingRequestMetrics
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type InitError = ();
    type Transform = IncomingRequestMetricsMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(IncomingRequestMetricsMiddleware { service }))
    }
}

pub struct IncomingRequestMetricsMiddleware<S> {
    service: S,
}

impl<S> Service<ServiceRequest> for IncomingRequestMetricsMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let start_time = Instant::now();

        let req_headers = get_headers(req.request());
        let req_path = get_path(req.request());
        let req_method = get_method(req.request());

        let fut = self.service.call(req);
        Box::pin(async move {
            match fut.await {
                Ok(response) => {
                    calculate_metrics(
                        response.response().error(),
                        response.status(),
                        get_headers(response.request()),
                        get_method(response.request()),
                        get_path(response.request()),
                        start_time,
                    );
                    Ok(response)
                }
                Err(err) => {
                    let err_resp_status = err.error_response().status();
                    calculate_metrics(
                        Some(&err),
                        err_resp_status,
                        req_headers,
                        req_method,
                        req_path,
                        start_time,
                    );
                    Err(err)
                }
            }
        })
    }
}
