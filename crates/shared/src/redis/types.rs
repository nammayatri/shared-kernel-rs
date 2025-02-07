/*  Copyright 2022-23, Juspay India Pvt Ltd
    This program is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License
    as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This program
    is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY
    or FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more details. You should have received a copy of
    the GNU Affero General Public License along with this program. If not, see <https://www.gnu.org/licenses/>.
*/

use chrono::{DateTime, Utc};
use error_stack::IntoReport;
use fred::{
    interfaces::{ClientLike, PubsubInterface},
    prelude::EventInterface,
    types::{ConnectHandle, Message, ReconnectPolicy, RedisConfig, RedisValue},
};
use log::info;
// use futures::{channel::mpsc::{self, UnboundedReceiver, UnboundedSender}, SinkExt};
use super::error::RedisError;
use serde::{de::DeserializeOwned, Deserialize};
use tokio::sync::mpsc;
use tokio::sync::{
    broadcast::Receiver,
    mpsc::{UnboundedReceiver, UnboundedSender},
};
use tracing::error;
use tracing::*;

#[derive(Debug, Deserialize, Clone)]
pub struct Point {
    pub lat: f64,
    pub lon: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub enum Ttl {
    TtlValue(i64),
    NoExpiry,
    NoKeyFound,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct RedisSettings {
    pub host: String,
    pub port: u16,
    pub cluster_enabled: bool,
    pub cluster_urls: Vec<String>,
    pub use_legacy_version: bool,
    pub pool_size: usize,
    pub reconnect_max_attempts: u32,
    /// Reconnect delay in milliseconds
    pub reconnect_delay: u32,
    /// TTL in seconds
    pub default_ttl: u32,
    /// TTL for hash-tables in seconds
    pub default_hash_ttl: u32,
    pub stream_read_count: u64,
    pub partition: usize,
    pub broadcast_channel_capacity: usize,
}

impl Default for RedisSettings {
    fn default() -> Self {
        RedisSettings {
            host: String::from("localhost"),
            port: 6379,
            cluster_enabled: false,
            cluster_urls: Vec::new(),
            use_legacy_version: false,
            pool_size: 10,
            reconnect_max_attempts: 5,
            reconnect_delay: 1000,
            default_ttl: 3600,
            default_hash_ttl: 3600,
            stream_read_count: 100,
            partition: 0,
            broadcast_channel_capacity: 32,
        }
    }
}

impl RedisSettings {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        host: String,
        port: u16,
        pool_size: usize,
        partition: usize,
        reconnect_max_attempts: u32,
        reconnect_delay: u32,
        default_ttl: u32,
        default_hash_ttl: u32,
        stream_read_count: u64,
        broadcast_channel_capacity: usize,
    ) -> Self {
        RedisSettings {
            host,
            port,
            partition,
            cluster_enabled: false,
            cluster_urls: Vec::new(),
            use_legacy_version: false,
            pool_size,
            reconnect_max_attempts,
            reconnect_delay,
            default_ttl,
            default_hash_ttl,
            stream_read_count,
            broadcast_channel_capacity,
        }
    }
}

pub struct RedisClient {
    pub client: fred::prelude::RedisClient,
}

impl std::ops::Deref for RedisClient {
    type Target = fred::prelude::RedisClient;
    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl RedisClient {
    pub async fn new(conf: RedisSettings) -> Result<Self, RedisError> {
        let (redis_config, reconnect_policy) = Self::get_config(&conf).await?;
        let client =
            fred::prelude::RedisClient::new(redis_config, None, None, Some(reconnect_policy));
        client.connect();
        client
            .wait_for_connect()
            .await
            .map_err(|err| RedisError::RedisConnectionError(err.to_string()))?;
        Ok(Self { client })
    }
    async fn get_config(
        conf: &RedisSettings,
    ) -> Result<(RedisConfig, ReconnectPolicy), RedisError> {
        let redis_connection_url = match conf.cluster_enabled {
            // Fred relies on this format for specifying cluster where the host port is ignored & only query parameters are used for node addresses
            // redis-cluster://username:password@host:port?node=bar.com:30002&node=baz.com:30003
            true => format!(
                "redis-cluster://{}:{}?{}",
                conf.host,
                conf.port,
                conf.cluster_urls
                    .iter()
                    .flat_map(|url| vec!["&", url])
                    .skip(1)
                    .collect::<String>()
            ),
            false => format!(
                "redis://{}:{}/{}", //URI Schema
                conf.host, conf.port, conf.partition
            ),
        };
        let mut config = fred::types::RedisConfig::from_url(&redis_connection_url)
            .into_report()
            .map_err(|err| RedisError::RedisConnectionError(err.to_string()))?;

        if !conf.use_legacy_version {
            config.version = fred::types::RespVersion::RESP3;
        }
        config.tracing = fred::types::TracingConfig::new(true);
        config.blocking = fred::types::Blocking::Error;
        let reconnect_policy = fred::types::ReconnectPolicy::new_constant(
            conf.reconnect_max_attempts,
            conf.reconnect_delay,
        );

        Ok((config, reconnect_policy))
    }

    pub async fn close_connection(&mut self) {
        match self.client.quit().await {
            Ok(_) => (),
            Err(err) => error!("[REDIS CLIENT CLOSE CONNECTION FAILED] => {:?}", err),
        }
    }
}

pub struct RedisConnectionPool {
    pub reader_pool: fred::prelude::RedisPool,
    pub writer_pool: fred::prelude::RedisPool,
    join_handles: Vec<ConnectHandle>,
}

impl RedisConnectionPool {
    /// Create a new Redis connection
    pub async fn new(
        conf: RedisSettings,
        replica_conf: Option<RedisSettings>,
    ) -> Result<Self, RedisError> {
        let (reader_pool, writer_pool, join_handles) = if let Some(replica_conf) = replica_conf {
            let (writer_pool, mut join_handles) = Self::instantiate(&conf).await?;
            let (reader_pool, reader_join_handles) = Self::instantiate(&replica_conf).await?;
            join_handles.extend(reader_join_handles);
            (reader_pool, writer_pool, join_handles)
        } else {
            let (writer_pool, mut join_handles) = Self::instantiate(&conf).await?;
            let (reader_pool, reader_join_handles) = Self::instantiate(&conf).await?;
            join_handles.extend(reader_join_handles);
            (reader_pool, writer_pool, join_handles)
        };

        Ok(Self {
            reader_pool,
            writer_pool,
            join_handles,
        })
    }
    async fn instantiate(
        conf: &RedisSettings,
    ) -> Result<(fred::prelude::RedisPool, Vec<fred::types::ConnectHandle>), RedisError> {
        let redis_connection_url = match conf.cluster_enabled {
            // Fred relies on this format for specifying cluster where the host port is ignored & only query parameters are used for node addresses
            // redis-cluster://username:password@host:port?node=bar.com:30002&node=baz.com:30003
            true => format!(
                "redis-cluster://{}:{}?{}",
                conf.host,
                conf.port,
                conf.cluster_urls
                    .iter()
                    .flat_map(|url| vec!["&", url])
                    .skip(1)
                    .collect::<String>()
            ),
            false => format!(
                "redis://{}:{}/{}", //URI Schema
                conf.host, conf.port, conf.partition
            ),
        };
        let mut config = fred::types::RedisConfig::from_url(&redis_connection_url)
            .into_report()
            .map_err(|err| RedisError::RedisConnectionError(err.to_string()))?;

        if !conf.use_legacy_version {
            config.version = fred::types::RespVersion::RESP3;
        }
        config.tracing = fred::types::TracingConfig::new(true);
        config.blocking = fred::types::Blocking::Error;
        let reconnect_policy = fred::types::ReconnectPolicy::new_constant(
            conf.reconnect_max_attempts,
            conf.reconnect_delay,
        );

        let mut performance_config = fred::types::PerformanceConfig::default();
        performance_config.broadcast_channel_capacity = conf.broadcast_channel_capacity;

        let pool = fred::prelude::RedisPool::new(
            config,
            Some(performance_config),
            None,
            Some(reconnect_policy),
            conf.pool_size,
        )
        .into_report()
        .map_err(|err| RedisError::RedisConnectionError(err.to_string()))?;

        let join_handles = pool.connect_pool();
        pool.wait_for_connect()
            .await
            .into_report()
            .map_err(|err| RedisError::RedisConnectionError(err.to_string()))?;

        Ok((pool, join_handles))
    }

    pub async fn close_connections(&mut self) {
        let _ = self.writer_pool.quit().await;
        let _ = self.reader_pool.quit().await;
        for handle in self.join_handles.drain(..) {
            match handle.await {
                Ok(Ok(_)) => (),
                Ok(Err(error)) => error!(%error),
                Err(error) => error!(%error),
            };
        }
    }

    pub async fn subscribe_channel<T>(
        &self,
        channel: &str,
    ) -> Result<mpsc::UnboundedReceiver<(String, T, DateTime<Utc>)>, RedisError>
    where
        T: DeserializeOwned + Send + 'static,
    {
        let (tx, mut rx): (
            UnboundedSender<(String, T, DateTime<Utc>)>,
            UnboundedReceiver<(String, T, DateTime<Utc>)>,
        ) = mpsc::unbounded_channel();

        let redis_connection = self.reader_pool.next();
        redis_connection.subscribe(channel).await.map_err(|e| {
            RedisError::GetFailed(format!(
                "Failed to subscribe to channel '{}': {}",
                channel, e
            ))
        })?;
        let mut message_stream: Receiver<Message> = redis_connection.message_rx();
        tokio::spawn(async move {
            loop {
                let res = message_stream.recv().await;
                match res {
                    Err(err) => error!(
                        "Error in receiving message from Redis, err : {}",
                        err.to_string()
                    ),
                    Ok(msg) => {
                        let channel_name = msg.channel.to_string();
                        match &msg.value {
                            RedisValue::String(val) => match serde_json::from_str::<T>(val) {
                                Ok(parsed) => {
                                    if let Err(err) = tx.send((channel_name, parsed, Utc::now())) {
                                        error!("Failed to send message to receiver: {}", err);
                                    }
                                }
                                Err(err) => {
                                    error!(
                                        "Deserialization error for channel '{}': {}",
                                        channel_name, err
                                    );
                                }
                            },
                            RedisValue::Null => {
                                error!("Received null value on channel '{}'", channel_name);
                            }
                            other => {
                                error!(
                                    "Unexpected RedisValue encountered on channel '{}': {:?}",
                                    channel_name, other
                                );
                            }
                        }
                    }
                }
            }
        });
        Ok(rx)
    }

    pub async fn subscribe_channel_as_str(
        &self,
        channel: &str,
    ) -> Result<mpsc::UnboundedReceiver<(String, String)>, RedisError> {
        let (tx, mut rx): (
            UnboundedSender<(String, String)>,
            UnboundedReceiver<(String, String)>,
        ) = mpsc::unbounded_channel();

        let redis_connection = self.reader_pool.next();
        redis_connection.subscribe(channel).await.map_err(|e| {
            RedisError::GetFailed(format!(
                "Failed to subscribe to channel '{}': {}",
                channel, e
            ))
        })?;
        let mut message_stream: Receiver<Message> = redis_connection.message_rx();
        tokio::spawn(async move {
            loop {
                let res = message_stream.recv().await;
                match res {
                    Err(err) => error!(
                        "Error in receiving message from Redis, err : {}",
                        err.to_string()
                    ),
                    Ok(msg) => {
                        let channel_name = msg.channel.to_string();
                        match &msg.value {
                            RedisValue::String(val) => {
                                if let Err(err) = tx.send((channel_name, val.to_string())) {
                                    error!("Failed to send message to receiver: {}", err);
                                }
                            }
                            RedisValue::Null => {
                                error!("Received null value on channel '{}'", channel_name);
                            }
                            other => {
                                error!(
                                    "Unexpected RedisValue encountered on channel '{}': {:?}",
                                    channel_name, other
                                );
                            }
                        }
                    }
                }
            }
        });
        Ok(rx)
    }
}
