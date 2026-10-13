use serde::de::DeserializeOwned;

/// A typed config section owned by a module or integration.
///
/// ```ignore
/// #[derive(Deserialize)]
/// #[serde(default)]
/// struct RedisConfig { url: String }
///
/// impl ConfigSection for RedisConfig {
///     const KEY: &'static str = "redis";
/// }
///
/// let redis: RedisConfig = ctx.config().section()?;
/// ```
pub trait ConfigSection: DeserializeOwned {
    /// Top-level key in the config (`redis` reads `redis:` in YAML and
    /// `REDIS__*` from the environment).
    const KEY: &'static str;
}
