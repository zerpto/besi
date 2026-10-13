//! The app's HTTP route table. The core adds `/health`, logging, and
//! shutdown handling; this file only lists the app's own endpoints, all
//! served under `/api`.
//!
//! To add an endpoint: create `<name>.rs` with an async handler, declare it
//! in `handlers/mod.rs`, and add a `.route(...)` line to `api_routes()`.
//! Paths there are relative to `/api` (`/hello/{name}` serves `/api/hello/{name}`).

use besi::axum::Router;
use besi::axum::routing::get;
use besi::{ApiRoutes, ApiState, Health};

use super::handlers;

/// Marker type naming this app's routes: `besi::ApiCommand<AppRoutes>`.
#[derive(Debug)]
pub struct AppRoutes;

impl ApiRoutes for AppRoutes {
    fn routes() -> Router<ApiState> {
        Router::new().nest("/api", api_routes())
    }

    async fn health(state: ApiState) -> Health {
        handlers::health::check(state).await
    }
}

fn api_routes() -> Router<ApiState> {
    Router::new().route("/hello/{name}", get(handlers::hello::hello))
}
