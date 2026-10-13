//! Helpers that turn files and the environment into config-rs sources.

use std::path::Path;

use ::config::{Environment, Map};

use crate::{Error, Result};

/// Separates nesting levels in env and `.env` keys.
const KEY_SEPARATOR: &str = "__";

/// Env-style source; `None` reads the process environment.
pub(super) fn env_source(source: Option<Map<String, String>>) -> Environment {
    Environment::default()
        .separator(KEY_SEPARATOR)
        .try_parsing(true)
        .source(source)
}

/// Parse a `.env` file into key/value pairs. A missing file yields no pairs.
pub(super) fn read_dotenv(path: &Path) -> Result<Map<String, String>> {
    if !path.is_file() {
        return Ok(Map::new());
    }
    let iter = dotenvy::from_path_iter(path)
        .map_err(|err| Error::msg(format!("{}: {err}", path.display())))?;
    iter.map(|item| item.map_err(|err| Error::msg(format!("{}: {err}", path.display()))))
        .collect()
}
