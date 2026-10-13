use serde::Deserialize;

/// Path parameters of `GET /api/hello/{name}`.
#[derive(Debug, Deserialize)]
pub struct HelloPath {
    pub name: String,
}
