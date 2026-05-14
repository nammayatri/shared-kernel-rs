// Integration tests for shared::db; per-test Postgres via testcontainers.

use std::pin::Pin;

use diesel::sql_query;
use diesel_async::RunQueryDsl;
use secrecy::SecretString;
use shared::db::config::{DbConfig, PoolConfig};
use shared::db::migrate::{migrate, MigrateOptions};
use shared::db::pool::DbPool;
use shared::db::replica::with_replica;
use shared::db::tx::with_transaction;
use testcontainers::ContainerAsync;
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::testcontainers::runners::AsyncRunner;

async fn boot_postgres() -> (ContainerAsync<Postgres>, DbConfig) {
    let container = Postgres::default()
        .start()
        .await
        .expect("postgres container failed to start");
    let host = container
        .get_host()
        .await
        .expect("container host")
        .to_string();
    let port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("container port");

    let cfg = DbConfig {
        host,
        port,
        database: "postgres".to_string(), // testcontainers default
        user: "postgres".to_string(),
        password: SecretString::from("postgres"),
        pool: PoolConfig::default(),
        replica: None,
    };
    (container, cfg)
}

#[tokio::test]
#[ignore]
async fn pool_round_trips_select_one() {
    let (_container, cfg) = boot_postgres().await;
    let pool = DbPool::from_config(&cfg).await.expect("build pool");
    let mut conn = pool.get().await.expect("checkout");

    let rows: usize = sql_query("SELECT 1")
        .execute(&mut *conn)
        .await
        .expect("SELECT 1");
    assert_eq!(rows, 1, "SELECT 1 should report 1 row affected");
}

#[tokio::test]
#[ignore]
async fn with_replica_falls_back_to_primary_when_unconfigured() {
    let (_container, cfg) = boot_postgres().await;
    let pool = DbPool::from_config(&cfg).await.expect("build pool");

    let result: i32 = with_replica(&pool, |conn| {
        Box::pin(async move {
            let n = sql_query("SELECT 1").execute(conn).await.unwrap_or(0);
            Ok(n as i32)
        })
    })
    .await
    .expect("with_replica returns");
    assert_eq!(result, 1);
}

#[tokio::test]
#[ignore]
async fn migrate_creates_table_and_is_idempotent() {
    let (_container, cfg) = boot_postgres().await;
    let pool = DbPool::from_config(&cfg).await.expect("build pool");

    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        temp.path().join("001_create_test.sql"),
        "CREATE TABLE migration_test_target (id INT PRIMARY KEY);",
    )
    .unwrap();
    std::fs::write(
        temp.path().join("002_add_col.sql"),
        "ALTER TABLE migration_test_target ADD COLUMN label TEXT;",
    )
    .unwrap();

    let report = migrate(&pool, temp.path(), &MigrateOptions::default())
        .await
        .expect("first migrate");
    assert_eq!(report.applied.len(), 2);
    assert_eq!(report.skipped.len(), 0);

    // Second run — everything should be skipped.
    let report = migrate(&pool, temp.path(), &MigrateOptions::default())
        .await
        .expect("second migrate");
    assert_eq!(report.applied.len(), 0);
    assert_eq!(report.skipped.len(), 2);

    // Verify the table actually exists and has the second column.
    let mut conn = pool.get().await.unwrap();
    sql_query("INSERT INTO migration_test_target (id, label) VALUES (1, 'ok')")
        .execute(&mut *conn)
        .await
        .expect("insert into migrated table");
}

#[tokio::test]
#[ignore]
async fn with_transaction_commits_on_success() {
    let (_container, cfg) = boot_postgres().await;
    let pool = DbPool::from_config(&cfg).await.expect("build pool");

    // Set up a table outside the transaction.
    let mut conn = pool.get().await.unwrap();
    sql_query("CREATE TABLE tx_test (id INT PRIMARY KEY)")
        .execute(&mut *conn)
        .await
        .unwrap();
    drop(conn);

    let result: Result<(), _> = with_transaction(&pool, |tx| {
        Box::pin(async move {
            sql_query("INSERT INTO tx_test (id) VALUES (1)")
                .execute(tx)
                .await
                .map_err(shared::db::error::DbError::from)?;
            Ok(())
        }) as Pin<Box<_>>
    })
    .await;
    result.expect("tx commit");

    let mut conn = pool.get().await.unwrap();
    let rows: Vec<TxRow> = sql_query("SELECT id FROM tx_test")
        .load::<TxRow>(&mut *conn)
        .await
        .expect("select after commit");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, 1);
}

#[tokio::test]
#[ignore]
async fn with_transaction_rolls_back_on_error() {
    let (_container, cfg) = boot_postgres().await;
    let pool = DbPool::from_config(&cfg).await.expect("build pool");

    let mut conn = pool.get().await.unwrap();
    sql_query("CREATE TABLE tx_rollback_test (id INT PRIMARY KEY)")
        .execute(&mut *conn)
        .await
        .unwrap();
    drop(conn);

    let result: Result<(), _> = with_transaction(&pool, |tx| {
        Box::pin(async move {
            sql_query("INSERT INTO tx_rollback_test (id) VALUES (1)")
                .execute(tx)
                .await
                .map_err(shared::db::error::DbError::from)?;
            // Force a rollback.
            Err(shared::db::error::DbError::Other)
        }) as Pin<Box<_>>
    })
    .await;
    assert!(result.is_err(), "transaction should propagate the error");

    let mut conn = pool.get().await.unwrap();
    let rows: Vec<TxRow> = sql_query("SELECT id FROM tx_rollback_test")
        .load::<TxRow>(&mut *conn)
        .await
        .expect("select after rollback");
    assert!(rows.is_empty(), "rollback should leave no rows");
}

#[derive(diesel::QueryableByName)]
struct TxRow {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    id: i32,
}
