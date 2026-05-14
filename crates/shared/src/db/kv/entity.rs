use serde::de::DeserializeOwned;
use serde::Serialize;

// Per-table KV contract. namma-dsl-rs emits one impl per declared model.
pub trait KvEntity: Sized + Send + Sync + Serialize + DeserializeOwned + 'static {
    type DomainType: From<Self> + Send + Sync + 'static;

    fn table_name() -> &'static str;
    fn primary_key(&self) -> String;
    fn secondary_indexes(&self) -> Vec<(&'static str, String)> {
        Vec::new()
    }

    fn redis_pk_key(pk: &str) -> String {
        format!("{}:pk:{}", Self::table_name(), pk)
    }

    fn redis_sk_key(field: &str, value: &str) -> String {
        format!("{}:sk:{}:{}", Self::table_name(), field, value)
    }
}
