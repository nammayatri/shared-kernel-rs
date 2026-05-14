use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct DbConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    pub password: SecretString,

    #[serde(default)]
    pub pool: PoolConfig,

    #[serde(default)]
    pub replica: Option<ReplicaConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PoolConfig {
    #[serde(default = "PoolConfig::default_min")]
    pub min: u32,
    #[serde(default = "PoolConfig::default_max")]
    pub max: u32,
    #[serde(default = "PoolConfig::default_idle_timeout_secs")]
    pub idle_timeout_secs: u64,
}

impl PoolConfig {
    fn default_min() -> u32 {
        1
    }
    fn default_max() -> u32 {
        16
    }
    fn default_idle_timeout_secs() -> u64 {
        600
    }
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            min: Self::default_min(),
            max: Self::default_max(),
            idle_timeout_secs: Self::default_idle_timeout_secs(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReplicaConfig {
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub pool: PoolConfig,
}

impl DbConfig {
    pub fn connection_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.user,
            self.password.expose_secret(),
            self.host,
            self.port,
            self.database,
        )
    }

    pub fn replica_connection_url(&self) -> Option<String> {
        self.replica.as_ref().map(|r| {
            format!(
                "postgres://{}:{}@{}:{}/{}",
                self.user,
                self.password.expose_secret(),
                r.host,
                r.port,
                self.database,
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_minimal_toml() {
        let toml = r#"
            host = "localhost"
            port = 5432
            database = "atlas_app"
            user = "postgres"
            password = "secret"
        "#;
        let cfg: DbConfig = toml::from_str(toml).unwrap();
        assert_eq!(cfg.host, "localhost");
        assert_eq!(cfg.port, 5432);
        assert_eq!(cfg.database, "atlas_app");
        assert_eq!(cfg.pool.min, 1);
        assert_eq!(cfg.pool.max, 16);
        assert!(cfg.replica.is_none());
    }

    #[test]
    fn deserializes_with_replica() {
        let toml = r#"
            host = "primary.local"
            port = 5432
            database = "atlas_app"
            user = "postgres"
            password = "secret"

            [pool]
            min = 4
            max = 32
            idle_timeout_secs = 60

            [replica]
            host = "replica.local"
            port = 5433
        "#;
        let cfg: DbConfig = toml::from_str(toml).unwrap();
        assert_eq!(cfg.pool.min, 4);
        assert_eq!(cfg.pool.max, 32);
        let replica = cfg.replica.as_ref().unwrap();
        assert_eq!(replica.host, "replica.local");
        assert_eq!(replica.port, 5433);
        assert_eq!(replica.pool.min, 1); // default
    }

    #[test]
    fn connection_url_includes_secret() {
        let toml = r#"
            host = "h"
            port = 5432
            database = "d"
            user = "u"
            password = "p"
        "#;
        let cfg: DbConfig = toml::from_str(toml).unwrap();
        assert_eq!(cfg.connection_url(), "postgres://u:p@h:5432/d");
    }
}
