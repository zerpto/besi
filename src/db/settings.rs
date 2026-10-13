use std::fmt;
use std::str::FromStr;
use std::time::Duration;

use serde::Deserialize;
use sqlx::postgres::{PgConnectOptions, PgSslMode};

use crate::{ConfigSection, Error, Result};

/// `database:` in config.yaml, or `DATABASE__HOST`, `DATABASE__PASSWORD`, ...
/// in .env / the environment. One key per setting; there is no URL form.
#[derive(Clone, Deserialize)]
#[serde(default)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    /// Empty means "don't send a password" (trust / peer auth).
    pub password: String,
    /// Database name.
    pub name: String,
    /// `disable`, `allow`, `prefer`, `require`, `verify-ca`, or `verify-full`.
    pub ssl_mode: String,
    pub max_connections: u32,
    pub min_connections: u32,
    /// How long to wait for a free connection (or a new one) before failing.
    pub acquire_timeout_secs: u64,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            host: "localhost".into(),
            port: 5432,
            username: "postgres".into(),
            password: String::new(),
            name: "postgres".into(),
            ssl_mode: "prefer".into(),
            max_connections: 10,
            min_connections: 0,
            acquire_timeout_secs: 5,
        }
    }
}

impl ConfigSection for DatabaseConfig {
    const KEY: &'static str = "database";
}

impl DatabaseConfig {
    /// sqlx connect options. Ignores `~/.pgpass`: the config is the only
    /// source of credentials.
    pub(super) fn connect_options(&self) -> Result<PgConnectOptions> {
        let ssl_mode = PgSslMode::from_str(&self.ssl_mode).map_err(|_| {
            Error::msg(format!(
                "database.ssl_mode: unknown value {:?}",
                self.ssl_mode
            ))
        })?;

        let options = PgConnectOptions::new_without_pgpass()
            .host(&self.host)
            .port(self.port)
            .username(&self.username)
            .database(&self.name)
            .ssl_mode(ssl_mode);

        Ok(if self.password.is_empty() {
            options
        } else {
            options.password(&self.password)
        })
    }

    pub(super) fn acquire_timeout(&self) -> Duration {
        Duration::from_secs(self.acquire_timeout_secs)
    }
}

/// Hand-written so the password never reaches logs or error output.
impl fmt::Debug for DatabaseConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DatabaseConfig")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("username", &self.username)
            .field(
                "password",
                &if self.password.is_empty() { "" } else { "***" },
            )
            .field("name", &self.name)
            .field("ssl_mode", &self.ssl_mode)
            .field("max_connections", &self.max_connections)
            .field("min_connections", &self.min_connections)
            .field("acquire_timeout_secs", &self.acquire_timeout_secs)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_hides_password() {
        let config = DatabaseConfig {
            password: "s3cret".into(),
            ..DatabaseConfig::default()
        };
        let debug = format!("{config:?}");
        assert!(!debug.contains("s3cret"), "{debug}");
        assert!(debug.contains("***"), "{debug}");
    }

    #[test]
    fn rejects_unknown_ssl_mode() {
        let config = DatabaseConfig {
            ssl_mode: "sometimes".into(),
            ..DatabaseConfig::default()
        };
        let err = config.connect_options().unwrap_err();
        assert!(err.to_string().contains("database.ssl_mode"), "{err}");
    }

    #[test]
    fn options_use_config_values() {
        let config = DatabaseConfig {
            host: "db.internal".into(),
            port: 6543,
            username: "app".into(),
            name: "app_dev".into(),
            ..DatabaseConfig::default()
        };
        let options = config.connect_options().unwrap();
        assert_eq!(options.get_host(), "db.internal");
        assert_eq!(options.get_port(), 6543);
        assert_eq!(options.get_username(), "app");
        assert_eq!(options.get_database(), Some("app_dev"));
    }
}
