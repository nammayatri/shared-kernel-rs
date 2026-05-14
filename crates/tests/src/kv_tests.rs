// KV-layer integration tests; spin both Postgres and Redis per test.

use std::pin::Pin;

use diesel::sql_query;
use diesel_async::RunQueryDsl;
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use shared::db::config::{DbConfig, PoolConfig};
use shared::db::error::DbError;
use shared::db::kv::{create_with_kv, find_one_with_kv, KvEntity, MeshConfig};
use shared::db::pool::DbPool;
use shared::redis::types::{RedisConnectionPool, RedisSettings};
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::redis::Redis;
use testcontainers_modules::testcontainers::runners::AsyncRunner;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct KvRow {
    id: String,
    label: String,
}

impl KvEntity for KvRow {
    type DomainType = KvRow;
    fn table_name() -> &'static str {
        "kv_test_row"
    }
    fn primary_key(&self) -> String {
        self.id.clone()
    }
}

async fn boot_postgres() -> (ContainerAsync<Postgres>, DbConfig) {
    let container = Postgres::default()
        .start()
        .await
        .expect("postgres container failed to start");
    let host = container.get_host().await.expect("host").to_string();
    let port = container.get_host_port_ipv4(5432).await.expect("port");
    let cfg = DbConfig {
        host,
        port,
        database: "postgres".to_string(),
        user: "postgres".to_string(),
        password: SecretString::from("postgres"),
        pool: PoolConfig::default(),
        replica: None,
    };
    (container, cfg)
}

async fn boot_redis() -> (ContainerAsync<Redis>, RedisConnectionPool) {
    let container = Redis::default()
        .with_tag("7-alpine")
        .start()
        .await
        .expect("redis container failed to start"); // need RESP3 (Redis 6+)
    let host = container.get_host().await.expect("host").to_string();
    let port = container.get_host_port_ipv4(6379).await.expect("port");
    let settings = RedisSettings::new(host, port, 4, 0, 3, 100, 0, 0, 10, 100);
    let pool = RedisConnectionPool::new(settings, None)
        .await
        .expect("redis pool");
    (container, pool)
}

async fn create_kv_test_table(pool: &DbPool) {
    let mut conn = pool.get().await.unwrap();
    sql_query("CREATE TABLE kv_test_row (id TEXT PRIMARY KEY, label TEXT NOT NULL)")
        .execute(&mut *conn)
        .await
        .expect("create kv_test_row");
}

#[tokio::test]
#[ignore]
async fn create_then_find_hits_redis_cache() {
    let (_pg, db_cfg) = boot_postgres().await;
    let (_rd, redis) = boot_redis().await;
    let pool = DbPool::from_config(&db_cfg).await.unwrap();
    create_kv_test_table(&pool).await;

    let mesh = MeshConfig {
        mesh_enabled: true,
        ..MeshConfig::default()
    };

    let row = KvRow {
        id: "u1".to_string(),
        label: "alice".to_string(),
    };

    create_with_kv::<KvRow, _>(&pool, &redis, &mesh, row.clone(), |conn, model| {
        Box::pin(async move {
            sql_query("INSERT INTO kv_test_row (id, label) VALUES ($1, $2)")
                .bind::<diesel::sql_types::Text, _>(model.id.clone())
                .bind::<diesel::sql_types::Text, _>(model.label.clone())
                .execute(conn)
                .await
                .map_err(DbError::from)?;
            Ok(())
        })
    })
    .await
    .expect("create_with_kv");

    let mut conn = pool.get().await.unwrap();
    sql_query("DELETE FROM kv_test_row WHERE id = 'u1'")
        .execute(&mut *conn)
        .await
        .expect("delete after create"); // wipe DB so only Redis can serve the read
    drop(conn);

    let key = KvRow::redis_pk_key("u1");
    let found = find_one_with_kv::<KvRow, _>(&pool, &redis, &mesh, &key, |_conn| {
        Box::pin(async move {
            panic!("DB fallback should not run when Redis has the row")
                as Result<Option<KvRow>, DbError>
        })
    })
    .await
    .expect("find_one_with_kv");

    assert_eq!(found, Some(row));
}

#[tokio::test]
#[ignore]
async fn find_falls_through_to_db_on_redis_miss() {
    let (_pg, db_cfg) = boot_postgres().await;
    let (_rd, redis) = boot_redis().await;
    let pool = DbPool::from_config(&db_cfg).await.unwrap();
    create_kv_test_table(&pool).await;

    let mesh = MeshConfig {
        mesh_enabled: true,
        ..MeshConfig::default()
    };

    let mut conn = pool.get().await.unwrap();
    sql_query("INSERT INTO kv_test_row (id, label) VALUES ('u2', 'bob')")
        .execute(&mut *conn)
        .await
        .unwrap();
    drop(conn);

    let key = KvRow::redis_pk_key("u2");
    let found = find_one_with_kv::<KvRow, _>(&pool, &redis, &mesh, &key, |conn| {
        Box::pin(async move {
            use diesel::sql_types::Text;

            #[derive(diesel::QueryableByName)]
            struct Row {
                #[diesel(sql_type = Text)]
                id: String,
                #[diesel(sql_type = Text)]
                label: String,
            }

            let rows = sql_query("SELECT id, label FROM kv_test_row WHERE id = $1")
                .bind::<Text, _>("u2")
                .load::<Row>(conn)
                .await
                .map_err(DbError::from)?;
            Ok(rows.into_iter().next().map(|r| KvRow {
                id: r.id,
                label: r.label,
            }))
        }) as Pin<Box<_>>
    })
    .await
    .expect("find_one_with_kv miss path");

    assert_eq!(
        found,
        Some(KvRow {
            id: "u2".to_string(),
            label: "bob".to_string()
        })
    );

    let cached: Option<KvRow> = redis.get_key(&key).await.unwrap_or(None);
    assert_eq!(
        cached,
        Some(KvRow {
            id: "u2".to_string(),
            label: "bob".to_string()
        }),
        "Redis should have been populated on miss"
    );
}

#[tokio::test]
#[ignore]
async fn mesh_disabled_bypasses_redis() {
    let (_pg, db_cfg) = boot_postgres().await;
    let (_rd, redis) = boot_redis().await;
    let pool = DbPool::from_config(&db_cfg).await.unwrap();
    create_kv_test_table(&pool).await;

    let mesh = MeshConfig::default(); // mesh_enabled = false

    let row = KvRow {
        id: "u3".to_string(),
        label: "carol".to_string(),
    };

    create_with_kv::<KvRow, _>(&pool, &redis, &mesh, row.clone(), |conn, model| {
        Box::pin(async move {
            sql_query("INSERT INTO kv_test_row (id, label) VALUES ($1, $2)")
                .bind::<diesel::sql_types::Text, _>(model.id.clone())
                .bind::<diesel::sql_types::Text, _>(model.label.clone())
                .execute(conn)
                .await
                .map_err(DbError::from)?;
            Ok(())
        })
    })
    .await
    .unwrap();

    let key = KvRow::redis_pk_key("u3");
    let cached: Option<KvRow> = redis.get_key(&key).await.unwrap_or(None);
    assert_eq!(cached, None, "mesh disabled should not write to Redis");
}

