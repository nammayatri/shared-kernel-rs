use std::time::Duration;

use bb8::Pool;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::AsyncPgConnection;

use crate::db::config::{DbConfig, PoolConfig};
use crate::db::error::DbError;

pub type DbConnManager = AsyncDieselConnectionManager<AsyncPgConnection>;
pub type DbPoolInner = Pool<DbConnManager>;
pub type PooledConn<'a> = bb8::PooledConnection<'a, DbConnManager>;

pub struct DbPool {
    pub(crate) primary: DbPoolInner,
    pub(crate) replica: Option<DbPoolInner>,
}

impl DbPool {
    pub async fn from_config(cfg: &DbConfig) -> Result<Self, DbError> {
        let primary = build_pool(&cfg.connection_url(), &cfg.pool).await?;
        let replica = if let Some(url) = cfg.replica_connection_url() {
            let replica_pool_cfg = cfg
                .replica
                .as_ref()
                .map(|r| r.pool.clone())
                .unwrap_or_default();
            Some(build_pool(&url, &replica_pool_cfg).await?)
        } else {
            None
        };
        Ok(Self { primary, replica })
    }

    pub async fn get(&self) -> Result<PooledConn<'_>, DbError> {
        self.primary.get().await.map_err(|_| DbError::PoolTimeout)
    }

    pub async fn get_replica(&self) -> Result<PooledConn<'_>, DbError> {
        match &self.replica {
            Some(p) => p.get().await.map_err(|_| DbError::PoolTimeout),
            None => self.get().await, // fall back to primary
        }
    }
}

async fn build_pool(url: &str, pool_cfg: &PoolConfig) -> Result<DbPoolInner, DbError> {
    let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(url);
    Pool::builder()
        .min_idle(Some(pool_cfg.min))
        .max_size(pool_cfg.max)
        .idle_timeout(Some(Duration::from_secs(pool_cfg.idle_timeout_secs)))
        .build(manager)
        .await
        .map_err(|_| DbError::ConnectionFailed)
}
