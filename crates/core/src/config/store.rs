use serde::de::DeserializeOwned;

use super::ConfigSection;
use crate::Result;

/// Loaded, merged configuration.
#[derive(Debug, Clone, Default)]
pub struct Config {
    inner: ::config::Config,
}

impl Config {
    pub(super) fn new(inner: ::config::Config) -> Self {
        Self { inner }
    }

    /// Read the value at a dotted key path, e.g. `"database.port"`.
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Result<T> {
        Ok(self.inner.get(key)?)
    }

    /// Read a typed section. If the section is absent entirely, it is built
    /// from an empty map, so `#[serde(default)]` fields still apply.
    pub fn section<S: ConfigSection>(&self) -> Result<S> {
        match self.inner.get(S::KEY) {
            Err(::config::ConfigError::NotFound(_)) => {
                Ok(::config::Config::default().try_deserialize()?)
            }
            other => Ok(other?),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Debug, Deserialize, PartialEq)]
    #[serde(default)]
    struct Db {
        port: u16,
    }

    impl Default for Db {
        fn default() -> Self {
            Self { port: 5432 }
        }
    }

    impl ConfigSection for Db {
        const KEY: &'static str = "database";
    }

    #[test]
    fn missing_section_uses_defaults() {
        assert_eq!(Config::default().section::<Db>().unwrap(), Db::default());
    }

    #[test]
    fn missing_key_errors() {
        assert!(Config::default().get::<u16>("database.port").is_err());
    }
}
