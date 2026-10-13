# ORM and migrations (SeaORM)

[SeaORM](https://www.sea-ql.org/SeaORM/) 2 on top of the existing PostgreSQL
pool ([database.md](database.md)), plus versioned migrations applied with
`app migrate`. Feature `orm` on `besi` (implies `db`).

**One pool, two APIs.** `db.orm()` is a SeaORM connection that wraps the same
sqlx pool as `db.pool()`; it is not a second pool. Raw sqlx and SeaORM can be
mixed freely, even in one function.

| Part | Where |
|---|---|
| `db.orm()`, `MigrateCommand` | [`crates/core/src/db/`](../crates/core/src/db/) (`pool.rs`, `migrate.rs`) |
| Entities (table models) | [`crates/entity/`](../crates/entity/): crate `entity`, one module per table |
| Migrations | [`crates/migration/`](../crates/migration/): crate `migration`, one file per migration |
| `migrate` command registration | [`crates/app/src/cmd/registry.rs`](../crates/app/src/cmd/registry.rs) |

Both crates are empty right now. [Worked example](#worked-example-a-note-table)
below walks through the first table end to end.

## `app migrate`

```sh
app migrate status             # every migration, Applied / Pending
app migrate up                 # apply all pending
app migrate up --steps 1       # apply only the next one
app migrate down               # roll back the most recent one
app migrate down --steps 3     # roll back the last three
app migrate fresh --yes        # DROP EVERY TABLE, then apply all
```

- It uses the same `database` config as everything else. Applied migrations
  are recorded in the `seaql_migrations` table, which is created on first use.
- `up` and `down` print each migration they applied or rolled back. When
  there's nothing to do they print `nothing to apply` / `nothing to roll back`.
- **`fresh` refuses without `--yes`**, before connecting:
  `` `migrate fresh` drops every table in database "app_dev"; re-run with --yes to confirm ``.
  It drops *every* table in the database, not only the ones migrations
  created. Never point it at production.
- Database errors exit 1, e.g. `database error: Failed to acquire connection
  from pool: Connection pool timed out` when Postgres is unreachable.

Migrations don't run automatically on startup. Run `app migrate up` as a
deploy step, before starting `app api` / `app cron`.

## Querying

Use SeaORM through the core's re-export, `besi::sea_orm`, so the app always
matches the core's version. `?` on a SeaORM error (`DbErr`) becomes
`besi::Error::Orm`, displayed as `database error: ...`.

**In a command** (`ctx.db()`):

```rust
use besi::sea_orm::{ActiveModelTrait, EntityTrait, QueryOrder, Set};
use entity::note;

let db = ctx.db()?.orm().clone();

note::ActiveModel {
    body: Set("buy milk".into()),
    ..Default::default()
}
.insert(&db)
.await?;

let notes = note::Entity::find()
    .order_by_desc(note::Column::CreatedAt)
    .all(&db)
    .await?;
```

**In a handler** (`state.db()`) and **in a cron job** (`ctx.db()` on
`JobContext`): the same, with `state.db()?.orm()`. In handlers, map errors to a
status code; don't return `DbErr` text to clients.

`DatabaseConnection` is cheap to clone (it wraps the shared pool), so
`.orm().clone()` is fine when a borrow of `ctx` gets in the way of
`ctx.out()`.

## Worked example: a `note` table

Every snippet here was compiled against this workspace (SeaORM 2.0).

### 1. Migration

`crates/migration/src/m20261012_000001_create_note.rs`:

```rust
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Note::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Note::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Note::Body).text().not_null())
                    .col(
                        ColumnDef::new(Note::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Note::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Note {
    Table,
    Id,
    Body,
    CreatedAt,
}
```

Register it in `crates/migration/src/lib.rs`, oldest first:

```rust
mod m20261012_000001_create_note;

// in Migrator::migrations()
Box::new(m20261012_000001_create_note::Migration),
```

Or generate both the file and the registration (see
[sea-orm-cli](#sea-orm-cli)). Then run `app migrate up`.

### 2. Entity

`crates/entity/src/note.rs`:

```rust
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "note")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub body: String,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
```

`crates/entity/src/lib.rs` (replacing the placeholder doc comment):

```rust
pub mod note;
pub mod prelude;
```

`crates/entity/src/prelude.rs`:

```rust
pub use super::note::Entity as Note;
```

### 3. Use it

`crates/app/src/cmd/commands/note.rs`, registered like any command
([commands.md](commands.md)):

```rust
use besi::sea_orm::{ActiveModelTrait, EntityTrait, QueryOrder, Set};
use besi::{Command, Context, Result};
use clap::Args;
use entity::note;

/// Add a note, then list all notes, newest first.
#[derive(Debug, Args)]
pub struct NoteCommand {
    /// Note text
    body: String,
}

impl Command for NoteCommand {
    async fn run(&self, ctx: &mut Context<'_>) -> Result<()> {
        let db = ctx.db()?.orm().clone();

        note::ActiveModel {
            body: Set(self.body.clone()),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        let notes = note::Entity::find()
            .order_by_desc(note::Column::CreatedAt)
            .all(&db)
            .await?;
        for n in notes {
            writeln!(ctx.out(), "{}  {}  {}", n.id, n.created_at, n.body)?;
        }
        Ok(())
    }
}
```

## sea-orm-cli

Optional: [`sea-orm-cli`](https://crates.io/crates/sea-orm-cli) generates
migrations and entities. Install with `cargo install sea-orm-cli@^2`.

**New migration** (creates the file and registers it in `lib.rs`):

```sh
sea-orm-cli migrate generate create_note -d crates/migration
```

- The CLI edits `crates/migration/src/lib.rs` with regexes. That file keeps
  SeaORM's standard layout for this reason. It's a crate root, not a `mod.rs`.
- The first generated `mod` line lands at the end of `lib.rs` (after the
  `impl`). That's valid Rust; move it up if you like. Later ones go after it.
- Don't write the list macro's name with an opening bracket anywhere else in
  `lib.rs`, comments included: the CLI's regex would match it and overwrite
  the wrong text.
- Apply with `app migrate up`, not `sea-orm-cli migrate up`, so the app's
  config is used.

**Entities from the live schema** (after `app migrate up`):

```sh
sea-orm-cli generate entity --lib -o crates/entity/src \
    -u postgres://USER:PASSWORD@HOST:PORT/NAME
```

This overwrites `crates/entity/src/` (`lib.rs`, `prelude.rs`, one module per
table). The CLI needs a URL; the app itself is configured with separate
`database.*` keys ([database.md](database.md#configuration)). Don't commit the
URL or put it in shell history with a real password.

## Testing

- **Without a database:** `MigrateCommand`'s tests in
  `crates/core/src/db/migrate.rs` cover the `fresh` refusal and the
  unreachable-database error, using the closed-port `Db` pattern from
  [database.md](database.md#testing).
- **With a database:** migrations and queries need a real Postgres. Run
  `app migrate fresh --yes` against a throwaway database (e.g. a Docker
  container) and exercise the commands.

## Enabling

```toml
# crates/app/Cargo.toml
besi = { workspace = true, features = ["orm"] }   # implies "db"
entity.workspace = true
migration.workspace = true
```

and register `Migrate(besi::MigrateCommand<migration::Migrator>)` in
`cmd/registry.rs`. Without `orm`: no SeaORM compiled, no `db.orm()`,
`besi::sea_orm` or `MigrateCommand`; raw sqlx via `db.pool()` still works.

SeaORM is built with `default-features = false` and: `macros`,
`sqlx-postgres`, `runtime-tokio-rustls`, `with-chrono`, `with-json`,
`with-uuid`. Add `with-rust_decimal` etc. in the root `Cargo.toml` when a
column needs them.

**Versions.** SeaORM 2.0 depends on sqlx 0.9, the same version as the `sqlx`
workspace dependency. Upgrade them together; if they diverge, `db.orm()` stops
compiling (two different `PgPool` types).

## Design notes

- `MigrateCommand<M>` uses sea-orm-migration's own `MigratorTrait` as its type
  parameter, the same contract as `ApiCommand<R>` / `CronCommand<J>`; the app
  dispatches it with `cmd.run(ctx).await`.
- `status` lists migrations via `get_migration_with_status` and prints them
  itself, rather than the library's `status()`, which only logs.
- Entities and migrations are separate crates (SeaORM's standard layout), so
  `sea-orm-cli` works unmodified and they don't depend on the app.
