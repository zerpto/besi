//! Database migrations, applied in order by `app migrate`.
//!
//! This file keeps SeaORM's standard layout so `sea-orm-cli migrate generate`
//! can register new migrations here automatically: it adds a module line and
//! an entry to the list in `migrations()`. Each migration is its own file in
//! `src/`, named `mYYYYMMDD_HHMMSS_<name>.rs`. See docs/orm.md.
//!
//! Don't write the list macro's name with an opening bracket anywhere else in
//! this file (comments included): the CLI finds the list with a regex.

pub use sea_orm_migration::prelude::*;

#[derive(Debug)]
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            // Oldest first. `sea-orm-cli migrate generate` maintains this list.
        ]
    }
}
