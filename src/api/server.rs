use axum::Router;
use axum::routing::get;
use tokio::net::TcpListener;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;

use super::{ApiConfig, ApiRoutes, ApiState, health};
use crate::{Error, Result};

/// Core `/health` + `R`'s routes, with request logging.
///
/// Panics at startup if `R::routes()` defines `/health`, which is axum's
/// behavior for overlapping routes. Customize the check with
/// [`ApiRoutes::health`] instead.
pub(super) fn router<R: ApiRoutes>(state: ApiState) -> Router {
    Router::new()
        .route("/health", get(health::handler::<R>))
        .merge(R::routes())
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .with_state(state)
}

pub(super) async fn serve(settings: &ApiConfig, app: Router) -> Result<()> {
    let listener = TcpListener::bind((settings.host.as_str(), settings.port))
        .await
        .map_err(|err| {
            Error::msg(format!(
                "cannot listen on {}:{}: {err}",
                settings.host, settings.port
            ))
        })?;
    tracing::info!("listening on http://{}", listener.local_addr()?);

    axum::serve(listener, app)
        .with_graceful_shutdown(crate::daemon::shutdown_signal())
        .await?;

    tracing::info!("server stopped");
    Ok(())
}

#[cfg(test)]
mod tests {
    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    use super::*;
    use crate::Config;
    use crate::api::Health;

    /// No routes, default health.
    struct Plain;
    impl ApiRoutes for Plain {
        fn routes() -> Router<ApiState> {
            Router::new()
        }
    }

    /// One route, default health.
    struct WithPing;
    impl ApiRoutes for WithPing {
        fn routes() -> Router<ApiState> {
            Router::new().route("/ping", get(|| async { "pong" }))
        }
    }

    /// Custom health: unhealthy.
    struct Down;
    impl ApiRoutes for Down {
        fn routes() -> Router<ApiState> {
            Router::new()
        }
        async fn health(_state: ApiState) -> Health {
            Health::unhealthy("database unreachable")
        }
    }

    /// Custom health that never finishes.
    struct Hung;
    impl ApiRoutes for Hung {
        fn routes() -> Router<ApiState> {
            Router::new()
        }
        async fn health(_state: ApiState) -> Health {
            std::future::pending().await
        }
    }

    async fn request<R: ApiRoutes>(uri: &str) -> (StatusCode, String) {
        let app = router::<R>(ApiState::new(Config::default()));
        let response = app
            .oneshot(Request::get(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, String::from_utf8(body.to_vec()).unwrap())
    }

    #[tokio::test]
    async fn health_defaults_to_ok() {
        let (status, body) = request::<Plain>("/health").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, r#"{"status":"ok"}"#);
    }

    #[tokio::test]
    async fn health_can_be_overridden() {
        let (status, body) = request::<Down>("/health").await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            body,
            r#"{"status":"unhealthy","reason":"database unreachable"}"#
        );
    }

    #[tokio::test(start_paused = true)]
    async fn hung_health_check_times_out() {
        let (status, body) = request::<Hung>("/health").await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert!(body.contains("timed out"), "{body}");
    }

    #[tokio::test]
    async fn app_routes_are_mounted() {
        assert_eq!(
            request::<WithPing>("/ping").await,
            (StatusCode::OK, "pong".into())
        );
    }

    #[tokio::test]
    async fn unknown_route_is_404() {
        let (status, _) = request::<Plain>("/nope").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }
}
