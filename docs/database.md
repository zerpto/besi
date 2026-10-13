# Database

PostgreSQL access through one shared connection pool, available everywhere:
`ctx.db()` in commands, `state.db()` in HTTP handlers. Built on
[sqlx](https://crates.io/crates/sqlx) 0.9 (raw SQL, no ORM).

**ORM and migrations:** SeaORM on this same pool (`db.orm()`) and the
`app migrate` command are covered in [orm.md](orm.md) (feature `orm`).

Code: [`crates/core/src/db/`](../crates/core/src/db/) (feature `db`).

| File | Contains |
|---|---|
| `settings.rs` | `DatabaseConfig`: the `database` config section |
| `pool.rs` | `Db`: the pool wrapper (`pool()`, `ping()`, `close()`) |

## Configuration

Section `database`, one key per setting (see [config.md](config.md) for
layering). There is no connection-URL form.

| Key | Env | Default | Notes |
|---|---|---|---|
| `database.host` | `DATABASE__HOST` | `localhost` | |
| `database.port` | `DATABASE__PORT` | `5432` | |
| `database.username` | `DATABASE__USERNAME` | `postgres` | |
| `database.password` | `DATABASE__PASSWORD` | empty | empty = don't send one (trust / peer auth) |
| `database.name` | `DATABASE__NAME` | `postgres` | database name |
| `database.ssl_mode` | `DATABASE__SSL_MODE` | `prefer` | `disable`, `allow`, `prefer`, `require`, `verify-ca`, `verify-full` |
| `database.max_connections` | `DATABASE__MAX_CONNECTIONS` | `10` | pool size |
| `database.min_connections` | `DATABASE__MIN_CONNECTIONS` | `0` | kept open when idle |
| `database.acquire_timeout_secs` | `DATABASE__ACQUIRE_TIMEOUT_SECS` | `5` | wait for a connection before failing |

```yaml
# config.yaml
database:
  host: localhost
  port: 5432
  username: app
  name: app_dev
```

```sh
# .env (keep secrets out of config.yaml)
DATABASE__PASSWORD=change-me
```

Notes:

- **Config wins.** `~/.pgpass` is ignored, and standard libpq variables
  (`PGHOST`, `PGUSER`, ...) never override the keys above. One exception: with
  `database.password` empty, no password is set, so sqlx falls back to
  `PGPASSWORD` if that is set in the environment.
- **The password is never printed.** `DatabaseConfig`'s `Debug` output shows
  `***`.
- **Bad values fail at startup, for every command.** An invalid `ssl_mode` or
  a wrong type stops even `app hello`, like any other config error. Only the
  *connection* is lazy.

## Lifecycle

- **One pool per process.** `besi::run` creates it after loading config,
  attaches it to the command's `Context`, and closes it when the command
  returns. `app api` hands the same pool to `ApiState`, so handlers share it.
- **Lazy.** Creating the pool opens no connection. Commands that never query
  (`app hello`) work with no Postgres running. A wrong host or password shows
  up on the first query.
- **Cheap to clone.** `Db` clones share one pool. Don't create more pools.
- **When unreachable**, a query waits up to `acquire_timeout_secs` then fails
  with `database error: pool timed out while waiting for an open connection`.
  sqlx doesn't say why (refused, DNS, auth); check host, port and credentials,
  or run `app db-check`.

## Using it

Queries are plain sqlx against `db.pool()`. Use sqlx through the core's
re-export, `besi::sqlx`, so the version always matches; the app has no direct
sqlx dependency.

**In a command:**

```rust
use besi::{Command, Context, Result};

impl Command for ListUsers {
    async fn run(&self, ctx: &mut Context<'_>) -> Result<()> {
        let names: Vec<String> = besi::sqlx::query_scalar("SELECT name FROM users")
            .fetch_all(ctx.db()?.pool())
            .await?;
        for name in names {
            writeln!(ctx.out(), "{name}")?;
        }
        Ok(())
    }
}
```

**In a handler:**

```rust
pub async fn show(
    State(state): State<ApiState>,
    Path(UserPath { id }): Path<UserPath>,
) -> Result<Json<User>, StatusCode> {
    let db = state.db().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let user = besi::sqlx::query_as::<_, User>("SELECT id, name FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(db.pool())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(user))
}
```

`query_as` needs `#[derive(besi::sqlx::FromRow)]` on the row type, which
requires adding the `derive` feature to sqlx in the root `Cargo.toml`.

Rules:

- **Always bind parameters** (`$1` + `.bind(...)`). Never build SQL with
  `format!` from input.
- `?` on a sqlx error becomes `besi::Error::Database` ("database error: ...").
  In handlers, don't return its text to the client; it can name tables and
  columns. Map it to a status code.
- `ctx.db()` / `state.db()` return `Result`: they are always set when run by
  `besi::run`, and error only in tests that didn't attach a pool.

## Commands

```sh
app db-check            # connect and print the Postgres server version
```

Exits 1 with the database error when it can't connect.

## Health

The app's `/health` check ([api.md](api.md#health-check)) pings the database
with `SELECT 1`, bounded to 1 s by `Db::ping`, under the API's 2 s health
timeout. Database down gives `503 {"status":"unhealthy","reason":"database unreachable"}`.

## Testing

No test needs a real database today. Patterns:

- **Code that needs a pool but shouldn't connect:** build one pointing at a
  closed port. `Db::connect_lazy` doesn't connect, so this is safe:

  ```rust
  let db = Db::connect_lazy(&DatabaseConfig {
      host: "127.0.0.1".into(),
      port: 1, // nothing listens here
      acquire_timeout_secs: 1,
      ..DatabaseConfig::default()
  })?;
  let mut ctx = Context::new(&mut buf, &config).with_db(db);
  ```

- **Failure paths** (unreachable, unhealthy) use exactly that; see
  `cmd/commands/db_check.rs` and `api/handlers/health.rs`.
- **Queries against real tables** need a running Postgres with the schema
  applied (`app migrate up`, see [orm.md](orm.md)).

`Db::connect_lazy` must run inside a tokio runtime, so use `#[tokio::test]`.

## Enabling

Feature `db` on `besi`, independent of `api`:

```toml
besi = { workspace = true, features = ["api", "db"] }   # this app
besi = { workspace = true, features = ["db"] }          # CLI only, with database
```

Without it, `ctx.db()`, `Db`, `besi::sqlx` and the `database` section handling
don't exist, and sqlx isn't compiled. `make lint` checks the core with each
feature alone and none.

sqlx is built with `runtime-tokio`, `tls-rustls` (pure Rust TLS, so the Linux
ARM build needs no OpenSSL) and `postgres`. No `macros` or `migrate` yet.

## SeaORM

See [orm.md](orm.md). With feature `orm`, `Db` also holds a SeaORM
`DatabaseConnection` wrapping this pool (`db.orm()`); raw sqlx via
`db.pool()` keeps working alongside.
