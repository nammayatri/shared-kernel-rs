use std::future::Future;
use std::pin::Pin;

use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, AsyncPgConnection};

use crate::db::error::DbError;
use crate::db::pool::DbPool;

const DEADLOCK_RETRY_LIMIT: usize = 3;

// Run f inside a Postgres transaction; retries on Deadlock.
pub async fn with_transaction<F, T>(pool: &DbPool, mut f: F) -> Result<T, DbError>
where
    F: for<'c> FnMut(
            &'c mut AsyncPgConnection,
        ) -> Pin<Box<dyn Future<Output = Result<T, DbError>> + Send + 'c>>
        + Send,
    T: Send + 'static,
{
    for attempt in 0..=DEADLOCK_RETRY_LIMIT {
        let mut conn = pool.get().await?;
        let result = conn
            .transaction::<T, DbError, _>(|tx| {
                let fut = f(tx);
                async move { fut.await }.scope_boxed()
            })
            .await;
        match result {
            Ok(value) => return Ok(value),
            Err(DbError::Deadlock) if attempt < DEADLOCK_RETRY_LIMIT => continue,
            Err(e) => return Err(e),
        }
    }
    Err(DbError::Deadlock)
}
