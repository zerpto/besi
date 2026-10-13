use serde::Serialize;

/// Body of `GET /api/hello/{name}`.
#[derive(Debug, Serialize)]
pub struct Greeting {
    message: String,
}

impl Greeting {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}
