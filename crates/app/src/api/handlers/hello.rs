use besi::ApiState;
use besi::axum::Json;
use besi::axum::extract::{Path, State};
use besi::axum::http::StatusCode;

use crate::api::requests::HelloPath;
use crate::api::responses::Greeting;
use crate::cmd::commands::hello::HelloConfig;

/// `GET /api/hello/{name}`: `{"message": "<greeting>, <name>!"}`.
pub async fn hello(
    State(state): State<ApiState>,
    Path(HelloPath { name }): Path<HelloPath>,
) -> Result<Json<Greeting>, (StatusCode, String)> {
    let config: HelloConfig = state
        .config()
        .section()
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    Ok(Json(Greeting::new(format!("{}, {name}!", config.greeting))))
}

#[cfg(test)]
mod tests {
    use besi::Config;
    use besi::axum::body::{Body, to_bytes};
    use besi::axum::http::Request;
    use tower::ServiceExt;

    use besi::ApiRoutes;

    use crate::api::AppRoutes;

    #[tokio::test]
    async fn greets_by_name() {
        let app = AppRoutes::routes().with_state(besi::ApiState::new(Config::default()));
        let response = app
            .oneshot(
                Request::get("/api/hello/Ferris")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body[..], br#"{"message":"Hello, Ferris!"}"#);
    }
}
