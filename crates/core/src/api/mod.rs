//! HTTP API server (feature `api`).
//!
//! The core owns the server: binding, request logging, graceful shutdown,
//! and `GET /health`. The application supplies its own routes by
//! implementing [`ApiRoutes`] and naming it in the command, which then
//! behaves like any other [`crate::Command`]:
//!
//! ```ignore
//! Api(besi::ApiCommand<AppRoutes>),               // in the Commands enum
//! Self::Api(cmd) => cmd.run(ctx),                 // in the dispatch match
//! ```

mod command;
mod health;
mod routes;
mod server;
mod settings;
mod state;

pub use command::ApiCommand;
pub use health::Health;
pub use routes::ApiRoutes;
pub use settings::ApiConfig;
pub use state::ApiState;
