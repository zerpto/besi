use std::path::PathBuf;

use ::config::{File, FileFormat, Map};

use super::Config;
use super::sources::{env_source, read_dotenv};
use crate::{Error, Result};

const DEFAULT_YAML: &str = "config.yaml";
const DEFAULT_DOTENV: &str = ".env";

/// Builds a [`Config`] from the YAML file, `.env` file, and environment.
#[derive(Debug, Clone)]
pub struct ConfigLoader {
    yaml: PathBuf,
    yaml_required: bool,
    dotenv: PathBuf,
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self {
            yaml: PathBuf::from(DEFAULT_YAML),
            yaml_required: false,
            dotenv: PathBuf::from(DEFAULT_DOTENV),
        }
    }
}

impl ConfigLoader {
    pub fn new() -> Self {
        Self::default()
    }

    /// Use this YAML file instead of the default, and fail if it is missing.
    pub fn with_yaml(mut self, path: impl Into<PathBuf>) -> Self {
        self.yaml = path.into();
        self.yaml_required = true;
        self
    }

    /// Use this `.env` file instead of the default. Skipped if missing.
    pub fn with_dotenv(mut self, path: impl Into<PathBuf>) -> Self {
        self.dotenv = path.into();
        self
    }

    pub fn load(&self) -> Result<Config> {
        self.load_with_env(None)
    }

    /// `env` replaces the process environment when set (used by tests).
    fn load_with_env(&self, env: Option<Map<String, String>>) -> Result<Config> {
        if self.yaml_required && !self.yaml.is_file() {
            return Err(Error::msg(format!(
                "config file not found: {}",
                self.yaml.display()
            )));
        }

        let inner = ::config::Config::builder()
            .add_source(
                File::from(self.yaml.as_path())
                    .format(FileFormat::Yaml)
                    .required(false),
            )
            .add_source(env_source(Some(read_dotenv(&self.dotenv)?)))
            .add_source(env_source(env))
            .build()?;

        Ok(Config::new(inner))
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use serde::Deserialize;

    use super::*;
    use crate::ConfigSection;

    #[derive(Debug, Deserialize, PartialEq)]
    #[serde(default)]
    struct Db {
        host: String,
        port: u16,
        name: String,
    }

    impl Default for Db {
        fn default() -> Self {
            Self {
                host: "localhost".into(),
                port: 5432,
                name: "app".into(),
            }
        }
    }

    impl ConfigSection for Db {
        const KEY: &'static str = "database";
    }

    /// Fresh directory with the given files written into it.
    fn fixture(name: &str, files: &[(&str, &str)]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("besi-config-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        for (file, content) in files {
            fs::write(dir.join(file), content).unwrap();
        }
        dir
    }

    fn loader(dir: &Path) -> ConfigLoader {
        ConfigLoader::new()
            .with_yaml(dir.join("config.yaml"))
            .with_dotenv(dir.join(".env"))
    }

    fn env(pairs: &[(&str, &str)]) -> Map<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn env_overrides_dotenv_overrides_yaml() {
        let dir = fixture(
            "layers",
            &[
                (
                    "config.yaml",
                    "database:\n  host: yaml-host\n  port: 1111\n  name: yaml-db\n",
                ),
                (".env", "DATABASE__PORT=2222\nDATABASE__NAME=dotenv-db\n"),
            ],
        );

        let config = loader(&dir)
            .load_with_env(Some(env(&[("DATABASE__NAME", "env-db")])))
            .unwrap();

        assert_eq!(
            config.section::<Db>().unwrap(),
            Db {
                host: "yaml-host".into(),
                port: 2222,
                name: "env-db".into(),
            }
        );
    }

    #[test]
    fn default_files_are_optional() {
        let dir = fixture("optional", &[]);
        let config = ConfigLoader::new()
            .with_dotenv(dir.join(".env"))
            .load_with_env(Some(Map::new()))
            .unwrap();
        assert_eq!(config.section::<Db>().unwrap(), Db::default());
    }

    #[test]
    fn explicit_yaml_is_required() {
        let dir = fixture("required", &[]);
        let err = loader(&dir).load_with_env(Some(Map::new())).unwrap_err();
        assert!(err.to_string().contains("config file not found"));
    }

    #[test]
    fn get_reads_dotted_key() {
        let dir = fixture("get", &[("config.yaml", "database:\n  port: 1111\n")]);
        let config = loader(&dir).load_with_env(Some(Map::new())).unwrap();
        assert_eq!(config.get::<u16>("database.port").unwrap(), 1111);
    }
}
