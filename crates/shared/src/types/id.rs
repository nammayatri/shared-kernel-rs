use std::convert::Infallible;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

// Phantom-typed newtype over String; Id<Seat> != Id<Account> at compile time.
pub struct Id<T> {
    inner: String,
    _phantom: PhantomData<fn() -> T>, // invariant, drops T from auto-traits
}

impl<T> Id<T> {
    pub fn new(s: impl Into<String>) -> Self {
        Self {
            inner: s.into(),
            _phantom: PhantomData,
        }
    }

    pub fn inner(&self) -> &str {
        &self.inner
    }

    pub fn into_inner(self) -> String {
        self.inner
    }
}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Id").field(&self.inner).finish()
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T> Eq for Id<T> {}

impl<T> Hash for Id<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl<T> fmt::Display for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.inner, f)
    }
}

impl<T> FromStr for Id<T> {
    type Err = Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(s))
    }
}

impl<T> From<String> for Id<T> {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl<T> From<&str> for Id<T> {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl<T> Serialize for Id<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.inner)
    }
}

impl<'de, T> Deserialize<'de> for Id<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(Self::new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Seat;
    struct Account;

    #[test]
    fn new_and_inner_roundtrip() {
        let id: Id<Seat> = Id::new("seat-123");
        assert_eq!(id.inner(), "seat-123");
    }

    #[test]
    fn from_str_infallible() {
        let id: Id<Seat> = "seat-123".parse().unwrap();
        assert_eq!(id.inner(), "seat-123");
    }

    #[test]
    fn display_matches_inner() {
        let id: Id<Seat> = Id::new("seat-123");
        assert_eq!(format!("{}", id), "seat-123");
    }

    #[test]
    fn equal_inners_are_equal_ids() {
        let a: Id<Seat> = Id::new("x");
        let b: Id<Seat> = Id::new("x");
        assert_eq!(a, b);
    }

    #[test]
    fn json_roundtrip() {
        let id: Id<Seat> = Id::new("seat-123");
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"seat-123\"");
        let parsed: Id<Seat> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, id);
    }

    #[test]
    fn phantom_type_distinguishes_at_compile_time() {
        let seat: Id<Seat> = Id::new("x");
        let account: Id<Account> = Id::new("x");
        assert_eq!(seat.inner(), account.inner());
    }
}
