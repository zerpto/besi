# Getting started

`besi` is a library. Your project owns the binary, the commands, the routes,
the jobs, the entities and the migrations; `besi` supplies the runner, the
`Command` contract, layered configuration and (behind features) the HTTP
server, PostgreSQL pool, SeaORM integration and cron scheduler.

## Add the dependency

```toml
[dependencies]
besi = { git = "https://github.com/zerpto/besi", branch = "main", features = ["api", "db", "orm", "cron"] }
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }

[dev-dependencies]
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Pin a `rev` or `tag` instead of `branch` for reproducible builds.

| Feature | Gives you | Docs |
|---|---|---|
| none | `Command`, `Context`, `App`, `run`, layered `Config` | [commands.md](commands.md), [config.md](config.md) |
| `api` | `ApiCommand<R>`: axum HTTP server with `/health` and request logging | [api.md](api.md) |
| `db` | `Db`: one lazy sqlx PostgreSQL pool per process | [database.md](database.md) |
| `orm` | `db.orm()` (SeaORM on the same pool), `MigrateCommand<M>`; implies `db` | [orm.md](orm.md) |
| `cron` | `CronCommand<J>`: job scheduler with config-driven schedules | [cron.md](cron.md) |

Your app depends on `clap` and `serde` directly because their derive macros
expand to `clap::...` / `serde::...` paths. Use the same major versions as
`besi` (clap 4, serde 1) so Cargo resolves a single copy. axum, sqlx and
SeaORM are re-exported (`besi::axum`, `besi::sqlx`, `besi::sea_orm`); use those
instead of direct dependencies.

## A minimal app

```rust
// src/main.rs
use std::process::ExitCode;

use besi::{App, Command, ConfigArgs, ConfigLoader, Context, Result};
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = env!("CARGO_BIN_NAME"), version, about)]
struct Cli {
    /// Global `--config <path>` flag.
    #[command(flatten)]
    config: ConfigArgs,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Print a greeting
    Hello(HelloCommand),
}

impl Command for Commands {
    async fn run(&self, ctx: &mut Context<'_>) -> Result<()> {
        match self {
            Self::Hello(cmd) => cmd.run(ctx).await,
        }
    }
}

impl Command for Cli {
    async fn run(&self, ctx: &mut Context<'_>) -> Result<()> {
        self.command.run(ctx).await
    }
}

impl App for Cli {
    fn config_loader(&self) -> ConfigLoader {
        self.config.loader()
    }
}

#[derive(Debug, clap::Args)]
struct HelloCommand {
    /// Who to greet
    #[arg(default_value = "world")]
    name: String,
}

impl Command for HelloCommand {
    async fn run(&self, ctx: &mut Context<'_>) -> Result<()> {
        writeln!(ctx.out(), "Hello, {}!", self.name)?;
        Ok(())
    }
}

fn main() -> ExitCode {
    besi::run::<Cli>()
}
```

`besi::run::<Cli>()` parses the CLI, loads config, creates the database pool
(feature `db`), starts the tokio runtime, runs the command, prints any error
as `error: ...` and returns the exit code. Don't add `#[tokio::main]`; `besi`
owns the runtime.

`App::config_loader` is optional. Without it (and without flattening
`ConfigArgs`) config comes from `./config.yaml`, `./.env` and the environment,
with no `--config` flag.

## Recommended project layout

`besi` doesn't impose a layout, but its features fit this one well. It keeps
`main.rs` small, gives every command, endpoint and job its own file, and puts
entities and migrations in separate crates so `sea-orm-cli` works unmodified.

```
Cargo.toml                 workspace: members, shared deps
config.example.yaml        documented non-secret settings (copy to config.yaml)
.env.example               secrets, commented out (copy to .env)
crates/
  app/                     the binary
    src/main.rs            `Cli` (implements `App`), calls besi::run
    src/cmd/registry.rs    `Commands` enum + dispatch
    src/cmd/commands/      one file per subcommand
    src/api/routes.rs      `AppRoutes: besi::ApiRoutes` (feature `api`)
    src/api/handlers/      async handlers, one file per endpoint group
    src/api/requests/      path/query/body types
    src/api/responses/     response body types
    src/cron/registry.rs   `AppJobs: besi::CronJobs` (feature `cron`)
    src/cron/jobs/         one file per job
  entity/                  SeaORM entities (feature `orm`)
  migration/               SeaORM migrations, `Migrator` in lib.rs (feature `orm`)
```

With every feature on, the command enum looks like this:

```rust
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Print a greeting
    Hello(hello::HelloCommand),
    /// Start the HTTP API server
    Api(besi::ApiCommand<AppRoutes>),
    /// Run scheduled jobs (`--list` to show them, `run <job>` to run one now)
    Cron(besi::CronCommand<AppJobs>),
    /// Manage database migrations (status, up, down, fresh)
    Migrate(besi::MigrateCommand<migration::Migrator>),
}

impl Command for Commands {
    async fn run(&self, ctx: &mut Context<'_>) -> Result<()> {
        match self {
            Self::Hello(cmd) => cmd.run(ctx).await,
            Self::Api(cmd) => cmd.run(ctx).await,
            Self::Cron(cmd) => cmd.run(ctx).await,
            Self::Migrate(cmd) => cmd.run(ctx).await,
        }
    }
}
```

The subcommand names (`api`, `cron`, `migrate`) are whatever you name the
variants; `besi` doesn't reserve any.

## Sample config

```yaml
# config.example.yaml: copy to config.yaml.
# Values here are overridden by .env, then by env vars.

# HTTP API (feature `api`). --host / --port flags override these.
api:
  host: 127.0.0.1
  port: 8080

# PostgreSQL (feature `db`). Put the password in .env, not here.
database:
  host: localhost
  port: 5432
  username: postgres
  name: postgres
  ssl_mode: prefer
  max_connections: 10

# Scheduled jobs (feature `cron`). Schedules are UTC; a job without one never runs.
cron:
  shutdown_timeout_secs: 30
  jobs:
    db_ping:
      schedule: "*/30 * * * * *"   # sec min hour day month weekday
      enabled: false
```

```sh
# .env.example: copy to .env. Overrides config.yaml; real env vars override this file.
# KEY__NESTED=value maps to key.nested in the config.
# API__HOST=0.0.0.0
# DATABASE__PASSWORD=change-me
# CRON__JOBS__DB_PING__SCHEDULE="*/30 * * * * *"
```

Every key and the layering rules are in [config.md](config.md).
