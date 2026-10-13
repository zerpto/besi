# besi

Rust console application. Cargo workspace with a reusable core framework and a
separate app crate that implements the commands.

## Layout

```
Cargo.toml                 workspace root: members, shared metadata/deps, release profile
Makefile                   dev/prod builds into dist/, plus test/lint/fmt shortcuts
config.example.yaml        sample config (copy to config.yaml)
.env.example               sample .env (copy to .env)
docs/                      design docs and references (config, api, commands, database, orm, cron)
crates/
  core/                    `besi` (lib): the framework, knows no concrete commands
    src/lib.rs             public API re-exports only
    src/error.rs           `Error` enum + `Result` alias
    src/cli/               running the application
      app.rs               `App` trait: top-level CLI (Parser + Command) + its ConfigLoader
      command.rs           `Command` trait: async fn run(&self, ctx: &mut Context) -> Result<()>
      context.rs           `Context`: passed to commands (stdout writer, config, db)
      runner.rs            `run::<A>()`: owns the tokio runtime; config, db pool, run, exit code
    src/config/            layered configuration (see docs/config.md)
      store.rs             `Config`: merged values, get / section
      section.rs           `ConfigSection` trait
      loader.rs            `ConfigLoader`: YAML < .env < env
      args.rs              `ConfigArgs`: global --config flag
      sources.rs           .env parsing, env source setup (internal)
    src/db/                PostgreSQL pool, feature `db` (see docs/database.md)
      settings.rs          `DatabaseConfig` section (host, port, username, password, ...)
      pool.rs              `Db`: sqlx pool wrapper (pool, orm, ping, close)
      migrate.rs           `MigrateCommand<M>`: `migrate status|up|down|fresh` (feature `orm`)
    src/api/               HTTP server, feature `api` (see docs/api.md)
      command.rs           `ApiCommand<R>`: the `api` subcommand (a plain `Command`)
      routes.rs            `ApiRoutes` trait: what the app supplies (routes, optional health())
      health.rs            `Health` + the core-owned /health handler (200/503, timeout)
      settings.rs          `ApiConfig` section (`api.host`, `api.port`)
      state.rs             `ApiState`: shared handler state (config)
      server.rs            router (/health + app routes + logging), serve
    src/cron/              job scheduler, feature `cron` (see docs/cron.md)
      command.rs           `CronCommand<J>`: `cron`, `cron --list`, `cron run <job>`
      jobs.rs              `CronJobs` trait: what the app supplies (its jobs)
      job.rs               `Job`: name + async fn
      context.rs           `JobContext`: name, config, db
      settings.rs          `CronConfig` / `JobSettings` (`cron` section)
      plan.rs              match jobs to config, validate names/schedules
      scheduler.rs         per-job loops, overlap skip, execute, shutdown wait
    src/daemon/            shared by `api` and `cron`
      logging.rs           tracing subscriber (stderr, RUST_LOG)
      shutdown.rs          Ctrl+C / SIGTERM
  app/                     `app` (bin): the application
    Cargo.toml             [[bin]] name = binary name (currently `app`)
    src/main.rs            top-level `Cli` parser (implements `App`), calls besi::run
    src/cmd/               CLI side of the app
      mod.rs               declarations, re-exports `Commands`
      registry.rs          `Commands` enum + dispatch
      commands/mod.rs      declares one module per subcommand
      commands/<name>.rs   one file per subcommand
    src/api/mod.rs         declares submodules, re-exports `AppRoutes`
    src/api/routes.rs      `AppRoutes: ApiRoutes`: route table (path -> handler, under /api)
    src/api/handlers/      async handlers, one file per endpoint group; health.rs = the app's health check
    src/api/requests/      types extracted from the request (path, query, body)
    src/api/responses/     types serialized into the response body
    src/cron/mod.rs        declares submodules, re-exports `AppJobs`
    src/cron/registry.rs   `AppJobs: CronJobs`: the job list
    src/cron/jobs/         one file per job: `pub async fn run(ctx: JobContext)`
  entity/                  `entity` (lib): SeaORM entities, one module per table (see docs/orm.md)
  migration/               `migration` (lib): SeaORM migrations; `Migrator` stays in lib.rs
                           (SeaORM layout, edited by `sea-orm-cli migrate generate`)
```

## Rules

- `crates/core` must stay command-agnostic. Never import from the `app` crate or add
  app-specific logic there; only general framework pieces (traits, context,
  errors, runner).
- Commands write output through `ctx.out()`, never `println!`, so they stay
  testable with a `Vec<u8>` buffer.
- Return `besi::Result`; use `Error::msg("...")` for user-facing failures.
  `?` works on `io::Error`.
- **`mod.rs` is for imports and exports only, in every crate.** Allowed:
  module docs (`//!`), `mod` / `pub mod` declarations, `use` and `pub use`.
  Nothing else: no `fn`, `struct`, `enum`, `trait`, `impl`, `const`, `static`
  or `type`. Put that code in a sibling file and re-export it (e.g.
  `cmd/registry.rs` holds `Commands`; `cmd/mod.rs` does
  `pub use registry::Commands;`).
- Organize `crates/core/src` by concept, not by kind (no `traits/` or
  `structs/` folders). Each concept is a directory module (`cli/`, `config/`)
  whose `mod.rs` follows the rule above; each submodule holds
  one type or one closely related group, with its unit tests. A concept that is
  one small type (like `error.rs`) may stay a single file until it grows.
  Don't name a file after its directory (`config/config.rs` trips clippy's
  `module_inception`).
- `lib.rs` re-exports every public item, so callers always write
  `besi::Item`, never `besi::config::loader::Item`. Moving files inside the
  core must not change the app crate.
- Prefer a popular, well-maintained third-party crate over hand-rolled code
  when one exists (check recent releases and download counts on crates.io).
  Current picks: `clap` (CLI), `config` (layered config), `dotenvy` (.env),
  `serde` (deserialization), `axum` + `tokio` (HTTP), `tower-http` +
  `tracing` (request logging), `tower` (tests), `sqlx` (PostgreSQL), `sea-orm` +
  `sea-orm-migration` (ORM and migrations, on the same sqlx pool; don't add
  another database crate), `croner` +
  `tokio-util` + `chrono` (cron). Reuse these before adding alternatives.
- Shared dependency versions go in `[workspace.dependencies]` in the root
  `Cargo.toml`; crates reference them with `dep.workspace = true`.

## Command contract

**Every subcommand implements `besi::Command`, and the app dispatches every one
of them with `cmd.run(ctx).await`. No exceptions, no side methods** like
`serve(ctx, extra)`. The runner and the `Commands` match only know `run`.

If a command needs something only the app can provide (routes, a job list,
plugins), it must not take it as an extra method or argument. Instead:

1. The core defines a small trait for that thing (like `ApiRoutes`).
2. The core command is generic over it: `MyCommand<P: MyProvider>`, holding
   `PhantomData<fn() -> P>` (`#[arg(skip)]`) so clap can still build it.
3. The app implements the trait on a marker type and names it in the enum:
   `My(besi::MyCommand<AppProvider>)`. The match arm stays `cmd.run(ctx).await`.

If a command cannot fit this shape, change the `Command` trait itself for all
commands rather than adding a one-off path.

## Adding a command

Full guide with examples: [docs/commands.md](docs/commands.md).

1. Create `crates/app/src/cmd/commands/<name>.rs` with a `#[derive(Debug, Args)]`
   struct that implements `besi::Command`.
2. In `crates/app/src/cmd/commands/mod.rs`: add `pub mod <name>;`.
3. In `crates/app/src/cmd/registry.rs`: add a variant in `Commands` (its
   doc comment becomes the help text) and a match arm.
4. Add a unit test in the command's file.

## Configuration

**Read [docs/config.md](docs/config.md) before touching config code or adding a
config section.**

In short: YAML (`config.yaml` or `--config`) < `.env` < env vars, loaded once
by `besi::run`, read in commands via `ctx.config()`. Env keys use `__` for
nesting, with no prefix (`DATABASE__PORT` sets `database.port`). YAML keys must
be lowercase. Each module defines its own `ConfigSection` struct
(`#[serde(default)]` + `Default`) and reads it with `ctx.config().section()`.
Never add module-specific config to `crates/core`.

## Cron

**Read [docs/cron.md](docs/cron.md) before touching cron code or adding a job.**

In short: `app cron` runs a scheduler owned by the core (`besi` feature `cron`).
The app lists jobs in `crates/app/src/cron/registry.rs` (`AppJobs: CronJobs`);
schedules come **only** from config (`cron.jobs.<name>.schedule`, UTC). Job
names are lowercase `snake_case`. Overlapping runs are skipped, failures and
panics are logged and don't stop the schedule, and shutdown waits up to
`cron.shutdown_timeout_secs`. Unknown job names in config fail startup.

## HTTP API

**Read [docs/api.md](docs/api.md) before touching API code or adding an
endpoint.**

In short: `app api` runs an axum server owned by the core (`besi` feature
`api`, opt-in via `features = ["api"]`). The core provides `/health` (the app
customizes the check by overriding `ApiRoutes::health`, never by defining the
route), request logging, graceful shutdown and the `api` config section; the app supplies
routes in `crates/app/src/api/` by implementing `besi::ApiRoutes` on `AppRoutes`;
the command is registered as `Api(besi::ApiCommand<AppRoutes>)` and dispatched
with a plain `cmd.run(ctx).await`. Use axum through `besi::axum`. Code
behind a feature must be `#[cfg(feature = "...")]`-gated in `lib.rs`.

## Database

**Read [docs/database.md](docs/database.md) before touching database code.**

In short: `besi` feature `db` gives one lazy sqlx PostgreSQL pool per process,
reached as `ctx.db()?.pool()` in commands and `state.db()?.pool()` in handlers.
Config is the `database` section with one key per setting (no URL). Use sqlx
via `besi::sqlx`, always bind parameters, never return database error text to
HTTP clients.

With feature `orm` (**read [docs/orm.md](docs/orm.md)**): `db.orm()` is a SeaORM
connection on the same pool; use SeaORM via `besi::sea_orm`. Entities live in
`crates/entity`, migrations in `crates/migration`, applied with
`app migrate up` (never automatically). Keep `sea-orm`'s sqlx version equal to
the `sqlx` workspace dependency.

## Renaming the binary

Change `name` under `[[bin]]` in `crates/app/Cargo.toml`. The help/usage text
follows automatically (`env!("CARGO_BIN_NAME")`), and the Makefile reads the
name from that file too.

## Commands

```sh
make dev                        # debug build   -> dist/dev/app
make prod                       # release build -> dist/prod/app (LTO, stripped)
make run ARGS="hello Ferris"    # cargo run with args
make run ARGS="api"             # start the HTTP API on :8080
make run ARGS="db-check"        # connect to Postgres, print its version
make run ARGS="cron --list"     # show cron jobs and their next run
make run ARGS="migrate status"  # migrations: Applied / Pending
make test                       # cargo test --workspace
make lint                       # clippy, warnings as errors (+ core without features)
make fmt                        # cargo fmt --all
make check                      # fmt check + lint + test
make clean                      # cargo clean + remove dist/
```

`make` with no target lists all targets.

## CI

`.github/workflows/build.yml` runs on push to `master`: `make check`, then
`make prod` natively on Linux amd64 and arm64 runners. Binaries are uploaded as
the `linux-amd64` and `linux-arm64` workflow artifacts.
