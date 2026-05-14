use diesel::result::{DatabaseErrorKind, Error as DieselError};

#[macros::add_error]
pub enum DbError {
    NotFound,
    UniqueViolation,
    Deadlock,
    ConnectionFailed,
    PoolTimeout,
    Migration,
    Serialization,
    Other,
}

impl From<DieselError> for DbError {
    fn from(e: DieselError) -> Self {
        match e {
            DieselError::NotFound => DbError::NotFound,
            DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => {
                DbError::UniqueViolation
            }
            DieselError::DatabaseError(DatabaseErrorKind::SerializationFailure, _) => {
                DbError::Deadlock
            }
            DieselError::SerializationError(_) | DieselError::DeserializationError(_) => {
                DbError::Serialization
            }
            _ => DbError::Other,
        }
    }
}
