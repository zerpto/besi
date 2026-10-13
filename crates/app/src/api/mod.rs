//! HTTP side of the app, split by role:
//!
//! - `handlers/`  async functions that do the work
//! - `requests/`  types extracted from the incoming request (path, query, body)
//! - `responses/` types serialized into the response body
//! - `routes.rs`  `AppRoutes`: the route table, path -> handler

mod handlers;
mod requests;
mod responses;
mod routes;

pub use routes::AppRoutes;
