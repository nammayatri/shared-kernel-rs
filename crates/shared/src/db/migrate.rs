use std::path::{Path, PathBuf};

use diesel::sql_query;
use diesel_async::RunQueryDsl;

use crate::db::error::DbError;
use crate::db::pool::DbPool;

pub struct MigrationReport {
    pub applied: Vec<String>,
    pub skipped: Vec<String>,
}

pub struct MigrateOptions {
    pub local_dev_index_rewrite: bool, // rewrite CONCURRENTLY -> blocking for local
}

impl Default for MigrateOptions {
    fn default() -> Self {
        Self {
            local_dev_index_rewrite: false,
        }
    }
}

// Apply every *.sql under dir in lexicographic order; idempotent via __migrations table.
pub async fn migrate(
    pool: &DbPool,
    migrations_dir: &Path,
    opts: &MigrateOptions,
) -> Result<MigrationReport, DbError> {
    ensure_migrations_table(pool).await?;

    let mut files = list_sql_files(migrations_dir)?;
    files.sort();

    let mut applied = Vec::new();
    let mut skipped = Vec::new();

    for path in files {
        let filename = path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or(DbError::Migration)?
            .to_string();

        let raw_sql = std::fs::read_to_string(&path).map_err(|_| DbError::Migration)?;
        let checksum = checksum_of(&raw_sql);
        let sql = if opts.local_dev_index_rewrite {
            rewrite_concurrent_indexes(&raw_sql)
        } else {
            raw_sql.clone()
        };

        if already_applied(pool, &filename, &checksum).await? {
            skipped.push(filename);
            continue;
        }

        apply_migration(pool, &filename, &checksum, &sql).await?;
        applied.push(filename);
    }

    Ok(MigrationReport { applied, skipped })
}

async fn ensure_migrations_table(pool: &DbPool) -> Result<(), DbError> {
    let mut conn = pool.get().await?;
    sql_query(
        "CREATE TABLE IF NOT EXISTS __migrations (\
            filename TEXT NOT NULL,\
            checksum TEXT NOT NULL,\
            applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),\
            PRIMARY KEY (filename, checksum)\
        )",
    )
    .execute(&mut *conn)
    .await
    .map_err(DbError::from)?;
    Ok(())
}

#[derive(diesel::QueryableByName)]
struct MigrationRow {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    count: i64,
}

async fn already_applied(pool: &DbPool, filename: &str, checksum: &str) -> Result<bool, DbError> {
    use diesel::sql_types::Text;

    let mut conn = pool.get().await?;
    let rows = sql_query(
        "SELECT COUNT(*) AS count FROM __migrations WHERE filename = $1 AND checksum = $2",
    )
    .bind::<Text, _>(filename)
    .bind::<Text, _>(checksum)
    .load::<MigrationRow>(&mut *conn)
    .await
    .map_err(DbError::from)?;
    Ok(rows.get(0).map(|c| c.count > 0).unwrap_or(false))
}

async fn apply_migration(
    pool: &DbPool,
    filename: &str,
    checksum: &str,
    sql: &str,
) -> Result<(), DbError> {
    use diesel::sql_types::Text;

    let mut conn = pool.get().await?;
    for stmt in split_statements(sql) {
        if stmt.trim().is_empty() {
            continue;
        }
        sql_query(stmt)
            .execute(&mut *conn)
            .await
            .map_err(DbError::from)?;
    }
    sql_query("INSERT INTO __migrations (filename, checksum) VALUES ($1, $2)")
        .bind::<Text, _>(filename)
        .bind::<Text, _>(checksum)
        .execute(&mut *conn)
        .await
        .map_err(DbError::from)?;
    Ok(())
}

fn list_sql_files(dir: &Path) -> Result<Vec<PathBuf>, DbError> {
    let entries = std::fs::read_dir(dir).map_err(|_| DbError::Migration)?;
    let mut out = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|_| DbError::Migration)?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("sql") {
            out.push(path);
        }
    }
    Ok(out)
}

fn checksum_of(s: &str) -> String {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

fn rewrite_concurrent_indexes(sql: &str) -> String {
    sql.replace("CREATE INDEX CONCURRENTLY", "CREATE INDEX")
        .replace("create index concurrently", "create index")
}

fn split_statements(sql: &str) -> Vec<&str> {
    sql.split(';').collect() // naive; ok for generator-emitted SQL
}
