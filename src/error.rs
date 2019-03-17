use std::{fmt, io, path::PathBuf, result};

use failure::{Backtrace, Context, Fail};

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    ctx: Context<ErrorKind>,
}

impl Error {
    /// Return the kind of this error.
    pub fn kind(&self) -> &ErrorKind {
        self.ctx.get_context()
    }

    pub fn invalid_unicode_path(path: PathBuf) -> Self {
        ErrorKind::InvalidUnicodePath { path }.into()
    }

    pub fn unconfigured_email() -> Self {
        ErrorKind::UnconfiguredEmail.into()
    }
}

impl Fail for Error {
    fn cause(&self) -> Option<&dyn Fail> {
        self.ctx.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.ctx.backtrace()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.ctx.fmt(f)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Fail)]
pub enum ErrorKind {
    #[fail(display = "invalid unicode path: {:?}", path)]
    InvalidUnicodePath { path: PathBuf },

    #[fail(display = "email is not configured")]
    UnconfiguredEmail,

    #[fail(display = "error sending email: {}", error)]
    EmailError { error: String },

    #[fail(display = "no user home directory found")]
    NoHomeDirectory,

    #[fail(display = "chrono error: {}", error)]
    ChronoError { error: String },

    #[fail(display = "crossbeam error: {}", error)]
    CrossbeamError { error: String },

    #[fail(display = "sled error: {}", error)]
    SledError { error: String },

    #[fail(display = "serde error: {}", error)]
    SerdeError { error: String },

    #[fail(display = "error parsing toml: {}", error)]
    TomlError { error: String },

    #[fail(display = "actix send error: {}", error)]
    ActixSendError { error: String },

    #[fail(display = "io error: {}", error)]
    IoError { error: String },
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error::from(Context::new(kind))
    }
}
impl From<Context<ErrorKind>> for Error {
    fn from(ctx: Context<ErrorKind>) -> Error {
        Error { ctx }
    }
}

/// map from IO errors
impl From<io::Error> for Error {
    fn from(error: io::Error) -> Error {
        Error::from(Context::new(ErrorKind::IoError {
            error: error.to_string(),
        }))
    }
}

/// map from toml errors
impl From<toml::de::Error> for Error {
    fn from(error: toml::de::Error) -> Error {
        Error::from(Context::new(ErrorKind::TomlError {
            error: error.to_string(),
        }))
    }
}

/// map from actix errors
impl<T> From<actix::prelude::SendError<T>> for Error {
    fn from(error: actix::prelude::SendError<T>) -> Error {
        Error::from(Context::new(ErrorKind::ActixSendError {
            error: error.to_string(),
        }))
    }
}

/// map from crossbeam errors
impl<T> From<crossbeam::queue::PushError<T>> for Error {
    fn from(error: crossbeam::queue::PushError<T>) -> Error {
        Error::from(Context::new(ErrorKind::CrossbeamError {
            error: error.to_string(),
        }))
    }
}

/// map from email errors
impl From<lettre::smtp::error::Error> for Error {
    fn from(error: lettre::smtp::error::Error) -> Error {
        Error::from(Context::new(ErrorKind::EmailError {
            error: error.to_string(),
        }))
    }
}

/// map from sled errors
impl<T: fmt::Debug> From<pagecache::Error<T>> for Error {
    fn from(error: pagecache::Error<T>) -> Error {
        Error::from(Context::new(ErrorKind::SledError {
            error: format!("{}", error),
        }))
    }
}

/// map from serde errors
impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Error {
        Error::from(Context::new(ErrorKind::SerdeError {
            error: format!("{}", error),
        }))
    }
}

/// map from chrono errors
impl From<chrono::format::ParseError> for Error {
    fn from(error: chrono::format::ParseError) -> Error {
        Error::from(Context::new(ErrorKind::ChronoError {
            error: format!("{}", error),
        }))
    }
}
