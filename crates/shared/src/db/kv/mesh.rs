use std::time::Duration;

// Per-call KV tuning; mirrors Haskell MeshConfig in Beam/Functions.hs.
#[derive(Debug, Clone)]
pub struct MeshConfig {
    pub mesh_enabled: bool,
    pub redis_ttl: Duration,
    pub redis_key_prefix: String,
    pub kv_hard_killed: bool, // bypass reads without flushing
    pub force_drain_to_db: bool, // reserved for future async-drain modes
}

impl Default for MeshConfig {
    fn default() -> Self {
        Self {
            mesh_enabled: false,
            redis_ttl: Duration::from_secs(18_000), // 5h, matches Haskell redisTtl
            redis_key_prefix: String::new(),
            kv_hard_killed: false,
            force_drain_to_db: true,
        }
    }
}

impl MeshConfig {
    pub fn redis_ttl_secs(&self) -> u32 {
        self.redis_ttl.as_secs().min(u32::MAX as u64) as u32
    }

    pub fn prefixed(&self, key: &str) -> String {
        if self.redis_key_prefix.is_empty() {
            key.to_string()
        } else {
            format!("{}{}", self.redis_key_prefix, key)
        }
    }
}
