use std::fmt;
use std::io;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Config(::config::ConfigError),
    #[cfg(feature = "db")]
    Database(sqlx::Error),
    #[cfg(feature = "orm")]
    Orm(sea_orm::DbErr),
    /// A user-facing failure message.
    Message(String),
}

impl Error {
    pub fn msg(message: impl Into<String>) -> Self {
        Self::Message(message.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "I/O error: {err}"),
            Self::Config(err) => write!(f, "config error: {err}"),
            #[cfg(feature = "db")]
            Self::Database(err) => write!(f, "database error: {err}"),
            #[cfg(feature = "orm")]
            Self::Orm(err) => write!(f, "database error: {err}"),
            Self::Message(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Config(err) => Some(err),
            #[cfg(feature = "db")]
            Self::Database(err) => Some(err),
            #[cfg(feature = "orm")]
            Self::Orm(err) => Some(err),
            Self::Message(_) => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<::config::ConfigError> for Error {
    fn from(err: ::config::ConfigError) -> Self {
        Self::Config(err)
    }
}

#[cfg(feature = "db")]
impl From<sqlx::Error> for Error {
    fn from(err: sqlx::Error) -> Self {
        Self::Database(err)
    }
}

#[cfg(feature = "orm")]
impl From<sea_orm::DbErr> for Error {
    fn from(err: sea_orm::DbErr) -> Self {
        Self::Orm(err)
    }
}
