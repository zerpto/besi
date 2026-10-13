//! SeaORM entities, one module per table. Empty until the first table exists.
//!
//! Generate from the live schema (after `app migrate up`), which writes this
//! file plus one module per table and `prelude.rs`:
//!
//! ```sh
//! sea-orm-cli generate entity --lib -o crates/entity/src \
//!     -u postgres://USER:PASSWORD@HOST:PORT/NAME
//! ```
//!
//! Or write them by hand; see docs/orm.md.
