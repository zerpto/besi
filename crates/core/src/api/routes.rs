use std::future::Future;

use axum::Router;

use super::{ApiState, Health};

/// What the application supplies to the `api` command.
///
/// Implement this on a marker type and name it in the command:
/// `ApiCommand<AppRoutes>`. The core adds `/health`, logging, and graceful
/// shutdown around it.
pub trait ApiRoutes: 'static {
    /// The app's HTTP routes.
    fn routes() -> Router<ApiState>;

    /// What "healthy" means for this app, behind the core's `GET /health`.
    ///
    /// The core owns the route, the response format, the status codes (200 /
    /// 503) and a timeout; this only decides the outcome. The default always
    /// reports healthy ("the process is up"). Override it to check
    /// dependencies such as a database.
    fn health(_state: ApiState) -> impl Future<Output = Health> + Send {
        async { Health::Healthy }
    }
}
