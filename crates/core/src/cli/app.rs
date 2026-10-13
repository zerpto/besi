use clap::Parser;

use crate::{Command, ConfigLoader};

/// The top-level application: the parsed CLI, run by [`crate::run`].
pub trait App: Parser + Command {
    /// How to load config for this run. Usually built from flattened
    /// [`crate::ConfigArgs`]; the default ignores CLI flags.
    fn config_loader(&self) -> ConfigLoader {
        ConfigLoader::default()
    }
}
