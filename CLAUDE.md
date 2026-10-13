# besi

Rust library: a framework for async console applications. Apps depend on it
(usually as a git dependency) and supply their own commands, routes, jobs,
entities and migrations. This repo contains only the framework; there is no
binary here.

## Layout

```
Cargo.toml                 the `besi` package: deps, features
Makefile                   build/test/lint/fmt/doc shortcuts
docs/                      user docs (getting-started, commands, config, api, database, orm, cron)
src/
  lib.rs                   public API re-exports only
  error.rs                 `Error` enum + `Result` alias
  cli/                     running the application
    app.rs                 `App` trait: top-level CLI (Parser + Command) + its ConfigLoader
    command.rs             `Command` trait: async fn run(&self, ctx: &mut Context) -> Result<()>
    context.rs             `Context`: passed to commands (stdout writer, config, db)
    runner.rs              `run::<A>()`: owns the tokio runtime; config, db pool, run, exit code
  config/                  layered configuration (see docs/config.md)
    store.rs               `Config`: merged values, get / section
    section.rs             `ConfigSection` trait
    loader.rs              `ConfigLoader`: YAML < .env < env
    args.rs                `ConfigArgs`: global --config flag
    sources.rs             .env parsing, env source setup (internal)
  db/                      PostgreSQL pool, feature `db` (see docs/database.md)
    settings.rs            `DatabaseConfig` section (host, port, username, password, ...)
    pool.rs                `Db`: sqlx pool wrapper (pool, orm, ping, close)
    migrate.rs             `MigrateCommand<M>`: `migrate status|up|down|fresh` (feature `orm`)
  api/                     HTTP server, feature `api` (see docs/api.md)
    command.rs             `ApiCommand<R>`: the `api` subcommand (a plain `Command`)
    routes.rs              `ApiRoutes` trait: what the app supplies (routes, optional health())
    health.rs              `Health` + the /health handler (200/503, timeout)
    settings.rs            `ApiConfig` section (`api.host`, `api.port`)
    state.rs               `ApiState`: shared handler state (config, db)
    server.rs              router (/health + app routes + logging), serve
  cron/                    job scheduler, feature `cron` (see docs/cron.md)
    command.rs             `CronCommand<J>`: `cron`, `cron --list`, `cron run <job>`
    jobs.rs                `CronJobs` trait: what the app supplies (its jobs)
    job.rs                 `Job`: name + async fn
    context.rs             `JobContext`: name, config, db
    settings.rs            `CronConfig` / `JobSettings` (`cron` section)
    plan.rs                match jobs to config, validate names/schedules
    scheduler.rs           per-job loops, overlap skip, execute, shutdown wait
  daemon/                  shared by `api` and `cron`
    logging.rs             tracing subscriber (stderr, RUST_LOG)
    shutdown.rs            Ctrl+C / SIGTERM
```

## Rules

- **Stay app-agnostic.** No concrete commands, routes, jobs, entities,
  migrations or app-specific config sections here. Only general framework
  pieces (traits, context, errors, runner, generic commands). Anything an app
  must supply arrives through a trait (see the command contract below).
- Commands write output through `ctx.out()`, never `println!`, so they stay
  testable with a `Vec<u8>` buffer.
- Return `besi::Result`; use `Error::msg("...")` for user-facing failures.
  `?` works on `io::Error`.
- **`mod.rs` is for imports and exports only.** Allowed: module docs (`//!`),
  `mod` / `pub mod` declarations, `use` and `pub use`. Nothing else: no `fn`,
  `struct`, `enum`, `trait`, `impl`, `const`, `static` or `type`. Put that
  code in a sibling file and re-export it.
- Organize `src` by concept, not by kind (no `traits/` or `structs/` folders).
  Each concept is a directory module (`cli/`, `config/`) whose `mod.rs`
  follows the rule above; each submodule holds one type or one closely related
  group, with its unit tests. A concept that is one small type (like
  `error.rs`) may stay a single file until it grows. Don't name a file after
  its directory (`config/config.rs` trips clippy's `module_inception`).
- `lib.rs` re-exports every public item, so callers always write
  `besi::Item`, never `besi::config::loader::Item`. Moving files inside `src`
  must not change any caller.
- Every feature is opt-in (`default = []`). Code behind a feature is
  `#[cfg(feature = "...")]`-gated in `lib.rs`; `make lint` checks each feature
  alone, none, and all.
- Third-party crates apps write code against (axum, sqlx, SeaORM) are
  re-exported (`besi::axum`, `besi::sqlx`, `besi::sea_orm`) so apps match our
  versions. Changing their major version is a breaking change for apps.
- Prefer a popular, well-maintained third-party crate over hand-rolled code
  when one exists (check recent releases and download counts on crates.io).
  Current picks: `clap` (CLI), `config` (layered config), `dotenvy` (.env),
  `serde` (deserialization), `axum` + `tokio` (HTTP), `tower-http` +
  `tracing` (request logging), `tower` (tests), `sqlx` (PostgreSQL), `sea-orm` +
  `sea-orm-migration` (ORM and migrations, on the same sqlx pool; don't add
  another database crate), `croner` + `tokio-util` + `chrono` (cron). Reuse
  these before adding alternatives. Keep `sea-orm`'s sqlx version equal to the
  `sqlx` dependency.
- Docs in `docs/` are for app authors using besi. When a public API changes,
  update the matching doc. Examples there describe app code; app-side paths
  follow the recommended layout in `docs/getting-started.md`.

## Command contract

**Every subcommand implements `besi::Command`, and apps dispatch every one of
them with `cmd.run(ctx).await`. No exceptions, no side methods** like
`serve(ctx, extra)`. The runner and an app's `Commands` match only know `run`.

If a command needs something only the app can provide (routes, a job list,
migrations), it must not take it as an extra method or argument. Instead:

1. Define a small trait for that thing (like `ApiRoutes`).
2. Make the command generic over it: `MyCommand<P: MyProvider>`, holding
   `PhantomData<fn() -> P>` (`#[arg(skip)]`) so clap can still build it.
3. The app implements the trait on a marker type and names it in its enum:
   `My(besi::MyCommand<AppProvider>)`. The match arm stays `cmd.run(ctx).await`.

If a command cannot fit this shape, change the `Command` trait itself for all
commands rather than adding a one-off path.

## Topic docs

Read the relevant doc before touching that area:

- [docs/config.md](docs/config.md): YAML (`config.yaml` or `--config`) < `.env`
  < env vars, `__` for nesting, no prefix, lowercase YAML keys. Each module
  defines its own `ConfigSection` (`#[serde(default)]` + `Default`).
- [docs/api.md](docs/api.md): `ApiCommand<R>`, `/health` owned here (apps
  customize via `ApiRoutes::health`, never by defining the route), request
  logging, graceful shutdown, `api` section.
- [docs/database.md](docs/database.md): one lazy sqlx pool per process,
  `database` section with one key per setting (no URL). Never return database
  error text to HTTP clients.
- [docs/orm.md](docs/orm.md): `db.orm()` on the same pool, `MigrateCommand<M>`.
  Migrations never run automatically.
- [docs/cron.md](docs/cron.md): schedules only from config
  (`cron.jobs.<name>.schedule`, UTC), lowercase `snake_case` job names, overlap
  skip, panics logged, shutdown waits `cron.shutdown_timeout_secs`, unknown job
  names fail startup.

## Commands

```sh
make build      # cargo build --all-features
make test       # cargo test --all-features
make lint       # clippy, warnings as errors, per feature combination
make fmt        # cargo fmt
make doc        # cargo doc --all-features, opened in a browser
make check      # fmt check + lint + test (CI gate)
make clean      # cargo clean
```

`make` with no target lists all targets.

## CI

`.github/workflows/check.yml` runs `make check` on pushes to `main` and on
pull requests.
