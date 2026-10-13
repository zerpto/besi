# besi

A Rust framework for async console applications. Your app defines its
commands with clap and hands the top-level parser to `besi::run`; besi owns
the tokio runtime, layered configuration (YAML < `.env` < env vars) and exit
codes. Optional features add an HTTP API server, a PostgreSQL pool, SeaORM
with migrations, and a cron scheduler, each exposed as a ready-made command.

```toml
[dependencies]
besi = { git = "https://github.com/zerpto/besi", features = ["api", "db", "orm", "cron"] }
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
```

| Feature | Adds |
|---|---|
| `api` | `ApiCommand<R>`: axum server with `/health`, request logging, graceful shutdown |
| `db` | `Db`: one lazy sqlx PostgreSQL pool per process |
| `orm` | SeaORM on the same pool and `MigrateCommand<M>` (implies `db`) |
| `cron` | `CronCommand<J>`: job scheduler with schedules from config |

## Docs

- [Getting started](docs/getting-started.md): dependency, a minimal app, recommended project layout
- [Commands](docs/commands.md)
- [Configuration](docs/config.md)
- [HTTP API](docs/api.md)
- [Database](docs/database.md)
- [ORM and migrations](docs/orm.md)
- [Cron](docs/cron.md)

## Development

```sh
make check   # fmt check + clippy (every feature combination) + tests
make doc     # API docs with every feature
```
