use besi::{ApiState, Health};

use crate::cmd::commands::hello::HelloConfig;

/// What "healthy" means for this app, served at the core's `GET /health`.
///
/// Healthy when the `hello` config (read by the endpoints on every request)
/// is usable and the database answers a ping. Add checks here as the app
/// gains dependencies. Reasons are public, so they stay generic.
pub async fn check(state: ApiState) -> Health {
    let config = match state.config().section::<HelloConfig>() {
        Ok(config) => config,
        Err(_) => return Health::unhealthy("hello config is invalid"),
    };
    if let Health::Unhealthy(reason) = verify(&config) {
        return Health::Unhealthy(reason);
    }

    match state.db() {
        Ok(db) if db.ping().await.is_ok() => Health::Healthy,
        Ok(_) => Health::unhealthy("database unreachable"),
        Err(_) => Health::unhealthy("database not configured"),
    }
}

fn verify(config: &HelloConfig) -> Health {
    if config.greeting.trim().is_empty() {
        Health::unhealthy("hello.greeting is empty")
    } else {
        Health::Healthy
    }
}

#[cfg(test)]
mod tests {
    use besi::{Config, DatabaseConfig, Db};

    use super::*;

    #[tokio::test]
    async fn unhealthy_without_database() {
        let state = ApiState::new(Config::default());
        assert_eq!(
            check(state).await,
            Health::unhealthy("database not configured")
        );
    }

    #[tokio::test]
    async fn unhealthy_when_database_unreachable() {
        let db = Db::connect_lazy(&DatabaseConfig {
            host: "127.0.0.1".into(),
            port: 1,
            ..DatabaseConfig::default()
        })
        .unwrap();
        let state = ApiState::new(Config::default()).with_db(db);
        assert_eq!(
            check(state).await,
            Health::unhealthy("database unreachable")
        );
    }

    #[test]
    fn empty_greeting_is_unhealthy() {
        let config = HelloConfig {
            greeting: "  ".into(),
        };
        assert_eq!(
            verify(&config),
            Health::unhealthy("hello.greeting is empty")
        );
    }

    #[test]
    fn set_greeting_is_healthy() {
        let config = HelloConfig {
            greeting: "Hi".into(),
        };
        assert_eq!(verify(&config), Health::Healthy);
    }
}
