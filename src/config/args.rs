use std::path::PathBuf;

use clap::Args;

use super::ConfigLoader;

/// Global CLI flags for config. Flatten into the top-level parser:
///
/// ```ignore
/// #[command(flatten)]
/// config: besi::ConfigArgs,
/// ```
#[derive(Debug, Clone, Default, Args)]
pub struct ConfigArgs {
    /// Path to the YAML config file [default: config.yaml, optional]
    #[arg(long = "config", value_name = "PATH", global = true)]
    pub path: Option<PathBuf>,
}

impl ConfigArgs {
    pub fn loader(&self) -> ConfigLoader {
        match &self.path {
            Some(path) => ConfigLoader::new().with_yaml(path),
            None => ConfigLoader::new(),
        }
    }
}
