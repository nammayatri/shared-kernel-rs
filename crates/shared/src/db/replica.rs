use std::future::Future;
use std::pin::Pin;

use diesel_async::AsyncPgConnection;

use crate::db::error::DbError;
use crate::db::pool::DbPool;

// Per-call replica routing; equivalent to Haskell runInReplica.
pub async fn with_replica<F, T>(pool: &DbPool, f: F) -> Result<T, DbError>
where
    F: for<'c> FnOnce(
        &'c mut AsyncPgConnection,
    ) -> Pin<Box<dyn Future<Output = Result<T, DbError>> + Send + 'c>>,
    T: Send,
{
    let mut conn = pool.get_replica().await?;
    f(&mut conn).await
}
