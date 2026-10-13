# Building a command

Every subcommand is a struct that implements `besi::Command`. The app crate
owns the commands; the core crate knows none of them.

| Part | Where |
|---|---|
| `Command` trait | [`crates/core/src/cli/command.rs`](../crates/core/src/cli/command.rs) |
| `Context` (stdout writer, config, database pool) | [`crates/core/src/cli/context.rs`](../crates/core/src/cli/context.rs) |
| Your commands | [`crates/app/src/cmd/commands/`](../crates/app/src/cmd/commands/), one file per command |
| Module declarations | [`crates/app/src/cmd/commands/mod.rs`](../crates/app/src/cmd/commands/mod.rs) (imports/exports only) |
| `Commands` enum + dispatch | [`crates/app/src/cmd/registry.rs`](../crates/app/src/cmd/registry.rs) |

[`hello.rs`](../crates/app/src/cmd/commands/hello.rs) is a complete working example.

## The contract

```rust
pub trait Command {
    async fn run(&self, ctx: &mut Context<'_>) -> Result<()>;
}
```

- **Every command is dispatched with `cmd.run(ctx).await`.** No side methods, no extra
  arguments. If a command needs something only the app can provide, see
  [Commands that need app-provided things](#commands-that-need-app-provided-things).
- `run` is async and is driven on a tokio runtime by `besi::run`, so you can
  `.await` database or HTTP calls directly.
- Output goes through `ctx.out()`, never `println!`, so tests can capture it.
- Return `besi::Result<()>`. `?` works on `io::Error` and config errors. For
  user-facing failures use `Error::msg("...")`; the runner prints it and exits
  non-zero.

## Adding a command

### 1. Create `crates/app/src/cmd/commands/<name>.rs`

```rust
use besi::{Command, Context, Result};
use clap::Args;

#[derive(Debug, Args)]
pub struct GreetCommand {
    /// Who to greet
    #[arg(default_value = "world")]
    name: String,

    /// Shout the greeting
    #[arg(long)]
    loud: bool,
}

impl Command for GreetCommand {
    async fn run(&self, ctx: &mut Context<'_>) -> Result<()> {
        let msg = format!("Hello, {}!", self.name);
        let msg = if self.loud { msg.to_uppercase() } else { msg };
        writeln!(ctx.out(), "{msg}")?;
        Ok(())
    }
}
```

Arguments and flags are plain [clap](https://crates.io/crates/clap) derive
attributes on the struct. Doc comments on fields become the `--help` text.

### 2. Declare it in `mod.rs`, register it in `registry.rs`

```rust
// crates/app/src/cmd/commands/mod.rs: declarations only
pub mod greet;                          // 1. declare the module
```

```rust
// crates/app/src/cmd/registry.rs
use super::commands::{db_check, greet, hello};

pub enum Commands {
    /// Print a greeting             // 2. this doc comment is the help text
    Greet(greet::GreetCommand),
    // ...
}

impl Command for Commands {
    async fn run(&self, ctx: &mut Context<'_>) -> Result<()> {
        match self {
            Self::Greet(cmd) => cmd.run(ctx).await,   // 3. plain dispatch
            // ...
        }
    }
}
```

The subcommand name is the variant name in kebab-case (`Greet` -> `app greet`;
`DbMigrate` -> `app db-migrate`).

### 3. Add a unit test in the same file

Pass a `Vec<u8>` as the writer and a `Config` of your choosing:

```rust
#[cfg(test)]
mod tests {
    use besi::Config;

    use super::*;

    async fn run(cmd: GreetCommand) -> String {
        let mut buf = Vec::new();
        let config = Config::default();
        cmd.run(&mut Context::new(&mut buf, &config)).await.unwrap();
        String::from_utf8(buf).unwrap()
    }

    #[tokio::test]
    async fn greets_loudly() {
        let cmd = GreetCommand { name: "Ferris".into(), loud: true };
        assert_eq!(run(cmd).await, "HELLO, FERRIS!\n");
    }
}
```

### 4. Try it

```sh
make run ARGS="greet Ferris --loud"
make check            # fmt check + clippy + tests
```

## Arguments and flags

Arguments are fields on the command's `#[derive(Args)]` struct, declared with
clap's `#[arg(...)]` attribute. The field's type decides how it parses; the
attribute decides how it is written on the command line. Doc comments become
`--help` text.

| You want | Field | Command line |
|---|---|---|
| Required positional | `user: String` | `app cmd alice` |
| Optional positional | `user: Option<String>` | `app cmd` or `app cmd alice` |
| Positional with default | `#[arg(default_value = "world")] name: String` | `app cmd` or `app cmd Ferris` |
| Several positionals | `user1: String, user2: String` | `app cmd alice bob` |
| Variable number of positionals | `users: Vec<String>` | `app cmd alice bob carol` |
| Required option | `#[arg(long)] path: String` | `--path /user` or `--path=/user` |
| Optional option | `#[arg(long)] path: Option<String>` | omit it, or `--path=/user` |
| Option with default | `#[arg(long, default_value = "/")] path: String` | omit it, or `--path=/user` |
| Short alias | `#[arg(short, long)] path: String` | `-p /user` or `--path /user` |
| On/off switch | `#[arg(long)] verbose: bool` | `--verbose` |
| Repeatable option | `#[arg(long)] tag: Vec<String>` | `--tag a --tag b` |
| Typed value | `#[arg(long)] port: u16` | `--port 9000` (rejects non-numbers) |

### Options: `--path` and `--path=/user`

`#[arg(long)]` makes the flag name from the field name (`path` -> `--path`,
`dry_run` -> `--dry-run`). Clap accepts both `--path /user` and `--path=/user`
with no extra code.

```rust
#[derive(Debug, Args)]
pub struct ListCommand {
    /// Path to list
    #[arg(long, default_value = "/")]
    path: String,

    /// Print full details
    #[arg(short, long)]
    verbose: bool,
}
```

```sh
app list                       # path = "/"
app list --path /user          # path = "/user"
app list --path=/user -v       # path = "/user", verbose = true
```

Use `Option<T>` when "not given" is meaningful, for example to fall back to a
config value (see how `ApiCommand` treats `--port`).

### Positionals: `{user1} {user2}`

A field without `long`/`short` is positional, filled in declaration order.

```rust
#[derive(Debug, Args)]
pub struct CompareCommand {
    /// First user
    user1: String,

    /// Second user
    user2: String,
}
```

```sh
app compare alice bob          # user1 = "alice", user2 = "bob"
app compare alice              # error: <USER2> is required
```

For an open-ended list, use one `Vec<String>` field. It must be the last
positional. Add `required = true` if at least one value is mandatory:

```rust
/// Users to look up
#[arg(required = true)]
users: Vec<String>,
```

The placeholder in `--help` is the field name uppercased (`<USER1>`); change
it with `#[arg(value_name = "NAME")]`.

### Validation

- **Typed fields** (`u16`, `PathBuf`, `bool`, ...) are parsed and rejected by
  clap with a usage error before `run` is called.
- **Fixed choices:** derive `clap::ValueEnum` on an enum and use it as the
  field type.
- **Custom check:** `#[arg(value_parser = my_fn)]` where
  `fn my_fn(s: &str) -> Result<T, String>`.
- **Rules across fields** that clap can't express: check at the top of `run`
  and return `Error::msg("...")`.
- **Environment fallback:** don't use `#[arg(env = ...)]`. Config and env are
  layered by `besi` itself; see [Reading configuration](#reading-configuration).

### Testing arguments

Unit tests can construct the struct directly (as above), but to test parsing
use `try_parse_from`. The first item is the program name:

```rust
use clap::Parser;

#[derive(Parser)]
struct Wrapper {
    #[command(flatten)]
    cmd: ListCommand,
}

#[test]
fn parses_path_with_equals() {
    let w = Wrapper::try_parse_from(["app", "--path=/user"]).unwrap();
    assert_eq!(w.cmd.path, "/user");
}

#[test]
fn rejects_missing_positional() {
    assert!(Wrapper::try_parse_from(["app"]).is_err());
}
```

## Reading configuration

Give the command its own `ConfigSection` and read it with
`ctx.config().section()`. Never put command-specific config in `crates/core`.

```rust
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct GreetConfig {
    pub greeting: String,
}

impl Default for GreetConfig {
    fn default() -> Self {
        Self { greeting: "Hello".into() }
    }
}

impl ConfigSection for GreetConfig {
    const KEY: &'static str = "greet";
}

// in run():
let config: GreetConfig = ctx.config().section()?;
```

This is set by `greet.greeting` in YAML or `GREET__GREETING` in `.env` / the
environment. In tests, build the `Config` with the values you need instead of
`Config::default()`. See [config.md](config.md) for layering and the loader.

If a flag should override config (like `--port` for `api`), read the section
first, then apply the flag if it was given.

## Using the database

With the `db` feature, `ctx.db()?` is the process-wide PostgreSQL pool:

```rust
let n: i64 = besi::sqlx::query_scalar("SELECT count(*) FROM users")
    .fetch_one(ctx.db()?.pool())
    .await?;
```

In tests, attach a pool with `Context::new(&mut buf, &config).with_db(db)`.
See [database.md](database.md) for config, lifecycle and testing, and
[`db_check.rs`](../crates/app/src/cmd/commands/db_check.rs) for a working command.

## Subcommands of subcommands

Nest a `Subcommand` enum inside the command's args struct and dispatch with
`run` the same way:

```rust
#[derive(Debug, Args)]
pub struct DbCommand {
    #[command(subcommand)]
    action: DbAction,
}

#[derive(Debug, Subcommand)]
enum DbAction {
    /// Apply pending migrations
    Migrate(MigrateArgs),
}

impl Command for DbCommand {
    async fn run(&self, ctx: &mut Context<'_>) -> Result<()> {
        match &self.action {
            DbAction::Migrate(args) => args.run(ctx).await,
        }
    }
}
```

## Commands that need app-provided things

Some commands need something only the app can supply (routes, a job list,
plugins). Do not add a `serve(ctx, extra)`-style method. Instead:

1. The core defines a small trait for that thing (like `ApiRoutes`).
2. The core command is generic over it, `MyCommand<P: MyProvider>`, holding
   `PhantomData<fn() -> P>` marked `#[arg(skip)]` so clap can still build it.
3. The app implements the trait on a marker type and names it in the enum:
   `My(besi::MyCommand<AppProvider>)`. The match arm stays `cmd.run(ctx).await`.

[`ApiCommand<R>`](../crates/core/src/api/command.rs) is the reference
implementation; [api.md](api.md) describes it. Code behind a Cargo feature must
be `#[cfg(feature = "...")]`-gated in `crates/core/src/lib.rs`.

If a command truly cannot fit this shape, change the `Command` trait for all
commands rather than adding a one-off path.

## Checklist

- [ ] `#[derive(Debug, Args)]` struct implementing `besi::Command`
- [ ] Output via `ctx.out()`, errors via `besi::Result` / `Error::msg`
- [ ] `pub mod` line in `cmd/commands/mod.rs`; enum variant (with doc comment) and `cmd.run(ctx).await` arm in `cmd/registry.rs`
- [ ] Unit test in the command's file
- [ ] Config in its own `ConfigSection`, not in `crates/core`
- [ ] `make check` passes
