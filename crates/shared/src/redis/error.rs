/*  Copyright 2022-23, Juspay India Pvt Ltd
    This program is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License
    as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This program
    is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY
    or FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more details. You should have received a copy of
    the GNU Affero General Public License along with this program. If not, see <https://www.gnu.org/licenses/>.
*/

use actix_web::{
    http::{header::ContentType, StatusCode},
    HttpResponse, ResponseError,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorBody {
    #[serde(skip_serializing_if = "String::is_empty")]
    error_message: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub error_code: String,
}

#[macros::add_error]
pub enum RedisError {
    SerializationError(String),
    DeserializationError(String),
    RedisConnectionError(String),
    TtlFailed(String),
    SetFailed(String),
    SetExFailed(String),
    SetExpiryFailed(String),
    GetFailed(String),
    MGetFailed(String),
    DeleteFailed(String),
    SetHashFieldFailed(String),
    GetHashFieldFailed(String),
    GetAllHashFieldFailed(String),
    DeleteHashFieldFailed(String),
    DeleteHashFieldsFailed(String),
    RPushFailed(String),
    RPopFailed(String),
    LPopFailed(String),
    LRangeFailed(String),
    LLenFailed(String),
    NotFound(String),
    InvalidRedisEntryId(String),
    SubscribeError(String),
    PublishError(String),
    GeoAddFailed(String),
    ZAddFailed(String),
    ZremrangeByRankFailed(String),
    GeoSearchFailed(String),
    ZCardFailed(String),
    GeoPosFailed(String),
    ZRangeFailed(String),
    XAddFailed(String),
    XReadFailed(String),
    XDeleteFailed(String),
    ClusterNodesError(String),
    ClusterKeySlotError(String),
    ClusterShardsToNodeError(String),
}

impl RedisError {
    #[inline]
    fn error_message(&self) -> ErrorBody {
        ErrorBody {
            error_message: self.message(),
            error_code: self.code().to_owned(),
        }
    }

    #[inline]
    pub fn message(&self) -> String {
        match self {
            RedisError::SerializationError(err) => err.clone(),
            RedisError::DeserializationError(err) => err.clone(),
            RedisError::RedisConnectionError(err) => format!("Redis Connection Error : {err}"),
            RedisError::TtlFailed(err)
            | RedisError::SetFailed(err)
            | RedisError::SetExFailed(err)
            | RedisError::SetExpiryFailed(err)
            | RedisError::GetFailed(err)
            | RedisError::MGetFailed(err)
            | RedisError::DeleteFailed(err)
            | RedisError::SetHashFieldFailed(err)
            | RedisError::GetHashFieldFailed(err)
            | RedisError::DeleteHashFieldFailed(err)
            | RedisError::DeleteHashFieldsFailed(err)
            | RedisError::RPushFailed(err)
            | RedisError::RPopFailed(err)
            | RedisError::LPopFailed(err)
            | RedisError::LRangeFailed(err)
            | RedisError::LLenFailed(err)
            | RedisError::NotFound(err)
            | RedisError::InvalidRedisEntryId(err)
            | RedisError::SubscribeError(err)
            | RedisError::PublishError(err)
            | RedisError::GeoAddFailed(err)
            | RedisError::ZAddFailed(err)
            | RedisError::ZremrangeByRankFailed(err)
            | RedisError::GeoSearchFailed(err)
            | RedisError::ZCardFailed(err)
            | RedisError::GeoPosFailed(err)
            | RedisError::ZRangeFailed(err)
            | RedisError::XReadFailed(err) => format!("Redis Error : {err}"),
            _ => "Some Error Occured".to_owned(),
        }
    }

    #[inline]
    fn code(&self) -> &'static str {
        match self {
            RedisError::SerializationError(_) => "SERIALIZATION_ERROR",
            RedisError::DeserializationError(_) => "DESERIALIZATION_ERROR",
            RedisError::TtlFailed(_) => "TTL_FAILED",
            RedisError::SetFailed(_) => "SET_FAILED",
            RedisError::SetExFailed(_) => "SET_EX_FAILED",
            RedisError::SetExpiryFailed(_) => "SET_EXPIRY_FAILED",
            RedisError::GetFailed(_) => "GET_FAILED",
            RedisError::MGetFailed(_) => "MGET_FAILED",
            RedisError::DeleteFailed(_) => "DELETE_FAILED",
            RedisError::SetHashFieldFailed(_) => "SETHASHFIELD_FAILED",
            RedisError::GetHashFieldFailed(_) => "GETHASHFIELD_FAILED",
            RedisError::GetAllHashFieldFailed(_) => "GETALLHASHFIELD_FAILED",
            RedisError::DeleteHashFieldFailed(_) => "DELETEHASHFIELD_FAILED",
            RedisError::DeleteHashFieldsFailed(_) => "DELETEHASHFIELDS_FAILED",
            RedisError::RPushFailed(_) => "RPUSH_FAILED",
            RedisError::RPopFailed(_) => "RPOP_FAILED",
            RedisError::LPopFailed(_) => "LPOP_FAILED",
            RedisError::LRangeFailed(_) => "LRANGE_FAILED",
            RedisError::LLenFailed(_) => "LLEN_FAILED",
            RedisError::NotFound(_) => "NOT_FOUND",
            RedisError::InvalidRedisEntryId(_) => "INVALID_REDIS_ENTRY_ID",
            RedisError::RedisConnectionError(_) => "REDIS_CONNECTION_FAILED",
            RedisError::SubscribeError(_) => "SUBSCRIBE_FAILED",
            RedisError::PublishError(_) => "PUBLISH_FAILED",
            RedisError::GeoAddFailed(_) => "GEOADD_FAILED",
            RedisError::ZAddFailed(_) => "ZADD_FAILED",
            RedisError::ZremrangeByRankFailed(_) => "ZREMRANGEBYRANK_FAILED",
            RedisError::GeoSearchFailed(_) => "GEOSEARCH_FAILED",
            RedisError::ZCardFailed(_) => "ZCARD_FAILED",
            RedisError::GeoPosFailed(_) => "GEOPOS_FAILED",
            RedisError::ZRangeFailed(_) => "ZRANGE_FAILED",
            RedisError::XAddFailed(_) => "XADD_FAILED",
            RedisError::XReadFailed(_) => "XREAD_FAILED",
            RedisError::XDeleteFailed(_) => "XDEL_FAILED",
            RedisError::ClusterNodesError(_) => "CLUSTER_NODES_ERROR",
            RedisError::ClusterShardsToNodeError(_) => "CLUSTER_SHARDS_TO_NODE_ERROR",
            RedisError::ClusterKeySlotError(_) => "CLUSTER_KEY_SLOT_ERROR",
        }
    }
}

impl ResponseError for RedisError {
    #[inline]
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .json(self.error_message())
    }

    #[inline]
    fn status_code(&self) -> StatusCode {
        match self {
            RedisError::SerializationError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            RedisError::DeserializationError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            RedisError::TtlFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
            RedisError::SetFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
            RedisError::SetExFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
            RedisError::SetExpiryFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
            RedisError::GetFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
            RedisError::MGetFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
            RedisError::DeleteFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
            RedisError::SetHashFieldFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
            RedisError::GetHashFieldFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
            RedisError::GetAllHashFieldFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
            RedisError::DeleteHashFieldFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
            RedisError::DeleteHashFieldsFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
            RedisError::NotFound(_) => StatusCode::NOT_FOUND,
            RedisError::InvalidRedisEntryId(_) => StatusCode::BAD_REQUEST,
            RedisError::RedisConnectionError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            RedisError::SubscribeError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            RedisError::PublishError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            RedisError::GeoAddFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
            RedisError::ZAddFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
            RedisError::ZremrangeByRankFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
            RedisError::GeoSearchFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
            RedisError::ZCardFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
            RedisError::GeoPosFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
            RedisError::ZRangeFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
            RedisError::RPushFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
            RedisError::RPopFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
            RedisError::LPopFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
            RedisError::LRangeFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
            RedisError::LLenFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
            RedisError::XAddFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
            RedisError::XReadFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
            RedisError::XDeleteFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
            RedisError::ClusterNodesError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            RedisError::ClusterShardsToNodeError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            RedisError::ClusterKeySlotError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
