use std::future::Future;
use std::pin::Pin;

use diesel_async::AsyncPgConnection;
use serde::de::DeserializeOwned;

use crate::db::error::DbError;
use crate::db::kv::{KvEntity, MeshConfig};
use crate::db::pool::DbPool;
use crate::redis::types::RedisConnectionPool;

type DbFut<'c, T> = Pin<Box<dyn Future<Output = Result<T, DbError>> + Send + 'c>>;

pub async fn create_with_kv<E, F>(
    pool: &DbPool,
    redis: &RedisConnectionPool,
    mesh: &MeshConfig,
    model: E,
    db_insert: F,
) -> Result<(), DbError>
where
    E: KvEntity,
    F: for<'c> FnOnce(&'c mut AsyncPgConnection, &'c E) -> DbFut<'c, ()>,
{
    let mut conn = pool.get().await?;
    db_insert(&mut conn, &model).await?;
    if mesh.mesh_enabled {
        let pk = model.primary_key();
        let pk_key = mesh.prefixed(&E::redis_pk_key(&pk));
        let _ = redis.set_key(&pk_key, &model, mesh.redis_ttl_secs()).await; // best-effort cache
        for (field, value) in model.secondary_indexes() {
            let sk_key = mesh.prefixed(&E::redis_sk_key(field, &value));
            let _ = redis.set_key(&sk_key, &pk, mesh.redis_ttl_secs()).await;
        }
    }
    Ok(())
}

pub async fn find_one_with_kv<E, F>(
    pool: &DbPool,
    redis: &RedisConnectionPool,
    mesh: &MeshConfig,
    redis_key: &str,
    db_fallback: F,
) -> Result<Option<E::DomainType>, DbError>
where
    E: KvEntity + DeserializeOwned,
    F: for<'c> FnOnce(&'c mut AsyncPgConnection) -> DbFut<'c, Option<E>>,
{
    let full_key = mesh.prefixed(redis_key);
    if mesh.mesh_enabled && !mesh.kv_hard_killed {
        if let Ok(Some(cached)) = redis.get_key::<E>(&full_key).await {
            return Ok(Some(cached.into()));
        }
    }
    let mut conn = pool.get().await?;
    let row = db_fallback(&mut conn).await?;
    if let Some(ref model) = row {
        if mesh.mesh_enabled {
            let _ = redis.set_key(&full_key, model, mesh.redis_ttl_secs()).await; // populate
        }
    }
    Ok(row.map(Into::into))
}

pub async fn update_with_kv<E, F>(
    pool: &DbPool,
    redis: &RedisConnectionPool,
    mesh: &MeshConfig,
    redis_keys_to_invalidate: &[&str],
    db_update: F,
) -> Result<(), DbError>
where
    E: KvEntity,
    F: for<'c> FnOnce(&'c mut AsyncPgConnection) -> DbFut<'c, ()>,
{
    let mut conn = pool.get().await?;
    db_update(&mut conn).await?;
    if mesh.mesh_enabled {
        for key in redis_keys_to_invalidate {
            let _ = redis.delete_key(&mesh.prefixed(key)).await; // invalidate; repopulate on next read
        }
    }
    let _ = (std::marker::PhantomData::<E>,);
    Ok(())
}

pub async fn delete_with_kv<E, F>(
    pool: &DbPool,
    redis: &RedisConnectionPool,
    mesh: &MeshConfig,
    redis_keys_to_delete: &[&str],
    db_delete: F,
) -> Result<(), DbError>
where
    E: KvEntity,
    F: for<'c> FnOnce(&'c mut AsyncPgConnection) -> DbFut<'c, ()>,
{
    let mut conn = pool.get().await?;
    db_delete(&mut conn).await?;
    if mesh.mesh_enabled {
        for key in redis_keys_to_delete {
            let _ = redis.delete_key(&mesh.prefixed(key)).await;
        }
    }
    let _ = (std::marker::PhantomData::<E>,);
    Ok(())
}
