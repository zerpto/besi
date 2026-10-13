use besi::{Command, ConfigSection, Context, Result};
use clap::Args;
use serde::Deserialize;

#[derive(Debug, Args)]
pub struct HelloCommand {
    /// Who to greet
    #[arg(default_value = "world")]
    name: String,
}

/// `hello:` in config.yaml, or `HELLO__GREETING` in .env / the environment.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct HelloConfig {
    pub greeting: String,
}

impl Default for HelloConfig {
    fn default() -> Self {
        Self {
            greeting: "Hello".into(),
        }
    }
}

impl ConfigSection for HelloConfig {
    const KEY: &'static str = "hello";
}

impl Command for HelloCommand {
    async fn run(&self, ctx: &mut Context<'_>) -> Result<()> {
        let config: HelloConfig = ctx.config().section()?;
        writeln!(ctx.out(), "{}, {}!", config.greeting, self.name)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use besi::Config;

    use super::*;

    async fn run(name: &str) -> String {
        let mut buf = Vec::new();
        let config = Config::default();
        let cmd = HelloCommand {
            name: name.to_string(),
        };
        cmd.run(&mut Context::new(&mut buf, &config)).await.unwrap();
        String::from_utf8(buf).unwrap()
    }

    #[tokio::test]
    async fn greets_by_name() {
        assert_eq!(run("Ferris").await, "Hello, Ferris!\n");
    }
}
