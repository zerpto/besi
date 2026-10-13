# Configuration

Configuration is loaded once at startup by the core framework (`besi`), merged
from three layers, and handed to every command through `ctx.config()`. Each
module owns and reads its own typed section; the core never needs to know which
sections exist.

Code: [`crates/core/src/config/`](../crates/core/src/config/)

| File | Contains |
|---|---|
| `store.rs` | `Config` |
| `section.rs` | `ConfigSection` |
| `loader.rs` | `ConfigLoader` and the layering tests |
| `args.rs` | `ConfigArgs` |
| `sources.rs` | `.env` parsing and env source setup (internal) |
Working example: `HelloConfig` in
[`crates/app/src/cmd/commands/hello.rs`](../crates/app/src/cmd/commands/hello.rs).

## Layers and precedence

Lowest to highest priority. A later layer overrides individual keys from an
earlier one; keys it doesn't set are kept.

| # | Source | Default location | If missing |
|---|---|---|---|
| 1 | YAML file | `./config.yaml` | skipped |
|   | YAML via `--config <path>` | given path | **error** |
| 2 | `.env` file | `./.env` | skipped |
| 3 | Process environment variables | | |

Paths are relative to the current working directory, not the binary.

The `.env` file is read as plain key/value pairs. It does **not** modify the
process environment, which is why real env vars can still override it.

Example, all three layers present:

```yaml
# config.yaml
database:
  host: db.internal
  port: 5432
  name: app
```

```sh
# .env
DATABASE__PORT=6543
DATABASE__NAME=app_dev
```

```sh
DATABASE__NAME=app_test ./app ...
```

Result: `host = db.internal` (YAML), `port = 6543` (.env),
`name = app_test` (env).

## The `--config` flag

```sh
app --config path/to/other.yaml hello
app hello --config path/to/other.yaml   # global flag, works after the command too
```

It replaces the default `config.yaml` path. `.env` and env vars still apply on
top. A path given explicitly must exist; the app exits with
`error: config file not found: <path>` otherwise.

## Key naming

| Where | Form | Example for `database.max_conn` |
|---|---|---|
| YAML | nested maps, **lowercase** keys | `database:` → `max_conn: 10` |
| `.env` / env var | `SECTION__KEY`, any case | `DATABASE__MAX_CONN=10` |

Rules:

- **`__` (double underscore) is the nesting separator.** A single `_` stays part
  of the key name, so `MAX_CONN` is the key `max_conn`.
- **Env and `.env` keys are lowercased; YAML keys are not.** Always write YAML
  keys in lowercase `snake_case`. A YAML key like `Database:` or `maxConn:` is
  never matched by a section and is silently ignored.
- **There is no prefix.** Every env var is loaded, so unrelated ones like
  `PATH` and `HOME` become top-level keys `path` and `home`. They're harmless
  unless a section uses the same name, so don't name a section after a common
  env var (`path`, `home`, `user`, `shell`, `lang`, `term`, `pwd`, ...).

## Values

- Env and `.env` values are strings, parsed when possible: `10` becomes a
  number and `true`/`false` a bool. They still deserialize into `String`
  fields correctly (`GREETING=123` gives `"123"`).
- **An empty env var is a real value.** `DATABASE__HOST=` sets `host` to `""`;
  it does not fall back to the YAML value. Unset the variable instead.
- `.env` supports quotes and trailing comments:
  `GREETING="hello there" # comment`.
- Lists and maps can only be set from YAML. Env vars set single scalar values.

## Reading config in a command

### Typed sections (preferred)

Define a struct for your module's settings and tell the core which top-level
key it lives under:

```rust
use besi::{Command, ConfigSection, Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(default)] // missing keys, or a missing section, fall back to Default
pub struct RedisConfig {
    url: String,
    pool_size: u32,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".into(),
            pool_size: 8,
        }
    }
}

impl ConfigSection for RedisConfig {
    const KEY: &'static str = "redis"; // `redis:` in YAML, REDIS__URL in env
}

// in Command::run
let redis: RedisConfig = ctx.config().section()?;
```

How `section()` behaves:

| Situation | Result |
|---|---|
| Section present, all keys set | values from config |
| Section present, some keys missing | missing keys from `Default` (needs `#[serde(default)]`) |
| Section absent entirely | the struct's `Default` (needs `#[serde(default)]`) |
| Required field (no `#[serde(default)]`) missing | `Err(config error: missing field ...)` |
| Wrong type (e.g. `port: abc`) | `Err(config error: invalid type ...)` |

To make a setting mandatory, leave `#[serde(default)]` off that field (or the
struct). The command then fails with a clear error when it isn't configured.

### One-off values

```rust
let port: u16 = ctx.config().get("database.port")?;
```

This takes a dotted path. It errors if the key is missing, so prefer sections
for anything with a default.

## Extending: modules and third-party integrations

A new integration (a database client, an HTTP client, a third-party SDK) adds
config by:

1. defining its own `ConfigSection` struct with a unique `KEY`, in its own
   module or crate;
2. reading it with `ctx.config().section::<T>()` wherever it's needed.

No changes to `besi` are needed. The core only depends on `serde`, so a section
type can live in any crate that depends on `besi`.

Choose a `KEY` that is unique across the app and isn't a common env var name
(see [Key naming](#key-naming)). Document new sections in
`config.example.yaml` and `.env.example`.

## Best practices

### Keep secrets out of `config.yaml`

Passwords, API keys, tokens and signing keys go in `.env` or real environment
variables, never in YAML. Everything else (hosts, ports, pool sizes, feature
toggles) can live in `config.yaml`.

| Kind | Where | Example |
|---|---|---|
| Secret | `.env` (local) or env var (CI, containers, servers) | `DATABASE__PASSWORD=change-me` |
| Non-secret | `config.yaml` | `database.host: db.internal` |

Because env sits above YAML in the [layers](#layers-and-precedence), a secret
set in the environment always wins over any value left in a file.

- **Never commit `.env`.** `.gitignore` excludes `.env` and `.env.*` but keeps
  `.env.example`. Commit only `.env.example`, with placeholder values.
- **Treat `config.yaml` as committable.** If a file holds anything you
  wouldn't paste in a PR, it belongs in `.env`. Commit `config.example.yaml`,
  not a personal `config.yaml` (add it to `.gitignore` if yours differs per
  machine).
- **In production, use real env vars** from your secret store (CI secrets,
  Docker/Kubernetes secrets, systemd `Environment=`). Don't ship a `.env`
  file in images or artifacts.
- **Add every new secret to `.env.example`**, commented out, with a dummy
  value, so others know which keys exist:

  ```sh
  # DATABASE__PASSWORD=change-me
  ```

- **Don't pass secrets as CLI flags.** They show up in `ps`, shell history and
  CI logs. Read them through config instead.
- **Don't prefill secrets with real defaults.** In a section's `Default`, use
  an empty string or leave the field required (no `#[serde(default)]` on it),
  so a missing secret fails loudly instead of using a shared password.
- **Never print a secret.** Derive `Debug` is fine for plain settings, but for a
  section with a secret, write `Debug` by hand and mask it, as
  `DatabaseConfig` does (it prints `***`). The same goes for logs and error
  messages.
- **Rotate if leaked.** If a secret reached git history, change it. Removing
  the commit isn't enough.

### General

- **One section per module**, with `#[serde(default)]` and sensible defaults,
  so the app runs with an empty config.
- **Fail early on required settings.** Make a mandatory field required rather
  than defaulting it to a placeholder.
- **Document every key** in `config.example.yaml` (non-secrets) or
  `.env.example` (secrets) when you add a section.
- **Override per environment with env vars**, not by editing `config.yaml`
  (for example `DATABASE__HOST=db.prod ./app ...`).
- **Don't read `std::env` directly** in commands. Go through `ctx.config()` so
  every setting follows the same layering and is testable.

## Testing

**Commands:** pass an empty config. Sections fall back to their defaults.

```rust
let mut buf = Vec::new();
let config = besi::Config::default();
cmd.run(&mut Context::new(&mut buf, &config)).unwrap();
```

**The loader itself:** tests in `config/loader.rs` use the private
`ConfigLoader::load_with_env(Some(map))`, which replaces the process
environment with `map`. Tests never set real env vars: that's process-global
and racy under parallel tests, and with no prefix any var could collide.

## Error handling

Config problems surface as `besi::Error::Config` (wrapping
`config::ConfigError`) or `Error::Message` (missing explicit file, unreadable
`.env`). Loading happens before any command runs, so a bad file stops the app
with `error: ...` and exit code 1 instead of running with partial config.

## API reference

All in `besi`:

| Item | Purpose |
|---|---|
| `Config` | merged config; `get::<T>(key)`, `section::<S>()`; `Default` is empty |
| `ConfigSection` | trait for a typed section: `const KEY: &'static str` + `DeserializeOwned` |
| `ConfigLoader` | builds `Config`; `new()`, `with_yaml(path)` (makes it required), `with_dotenv(path)`, `load()` |
| `ConfigArgs` | clap args for the global `--config` flag; `loader()` turns it into a `ConfigLoader` |
| `App` | trait for the top-level CLI; `config_loader()` says how to load config (default: `ConfigLoader::default()`) |
| `Context::config()` | the loaded config, inside a command |

Startup flow (`besi::run::<A>()`):

```
A::parse()  →  app.config_loader().load()  →  Context::new(stdout, &config)  →  app.run(ctx)
```

The app opts in to the `--config` flag by flattening `ConfigArgs` into its CLI
and returning `self.config.loader()` from `App::config_loader`. See
[`crates/app/src/main.rs`](../crates/app/src/main.rs).

## Implementation notes

- **Libraries:** [`config`](https://crates.io/crates/config) (config-rs) merges
  the layers, with `default-features = false, features = ["yaml"]`.
  [`dotenvy`](https://crates.io/crates/dotenvy) parses `.env`.
- **How `.env` fits between YAML and env:** both use config-rs's `Environment`
  source with the same settings (`separator("__")`, `try_parsing(true)`). The
  `.env` one is given the parsed file as a custom map via
  `Environment::source(Some(map))`. The env one is given `None`, which reads the
  real process environment.
- **Name clash:** inside the core, the crate's own module is `config` and so is
  the dependency, so the dependency is always written `::config::...`. For the
  same reason, `Config` lives in `store.rs`, not `config/config.rs`.
