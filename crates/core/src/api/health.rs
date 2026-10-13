use std::time::Duration;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::Serialize;

use super::{ApiRoutes, ApiState};

/// How long a health check may take before it counts as unhealthy, so a
/// hung dependency can't hang the probe too.
const TIMEOUT: Duration = Duration::from_secs(2);

/// Outcome of a health check, returned by [`ApiRoutes::health`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Health {
    Healthy,
    /// Unhealthy, with a reason that is logged and returned in the response
    /// body. Don't put secrets or internal addresses in it.
    Unhealthy(String),
}

impl Health {
    pub fn unhealthy(reason: impl Into<String>) -> Self {
        Self::Unhealthy(reason.into())
    }
}

#[derive(Serialize)]
pub(super) struct Body {
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}

/// `GET /health`: 200 `{"status":"ok"}` when healthy, otherwise
/// 503 `{"status":"unhealthy","reason":"..."}`. The check itself is `R::health`.
pub(super) async fn handler<R: ApiRoutes>(
    State(state): State<ApiState>,
) -> (StatusCode, Json<Body>) {
    let health = tokio::time::timeout(TIMEOUT, R::health(state))
        .await
        .unwrap_or_else(|_| Health::unhealthy("health check timed out"));

    match health {
        Health::Healthy => (
            StatusCode::OK,
            Json(Body {
                status: "ok",
                reason: None,
            }),
        ),
        Health::Unhealthy(reason) => {
            tracing::warn!("health check failed: {reason}");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(Body {
                    status: "unhealthy",
                    reason: Some(reason),
                }),
            )
        }
    }
}
