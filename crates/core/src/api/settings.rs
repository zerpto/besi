use serde::Deserialize;

use crate::ConfigSection;

/// `api:` in config.yaml, or `API__HOST` / `API__PORT` in .env / the
/// environment. The `--host` / `--port` flags override both.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ApiConfig {
    pub host: String,
    pub port: u16,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 8080,
        }
    }
}

impl ConfigSection for ApiConfig {
    const KEY: &'static str = "api";
}
