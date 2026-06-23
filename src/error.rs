use std::fmt;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug)]
pub enum AppError {
    Usage(String),
    Io(std::io::Error),
    Json(serde_json::Error),
    Remote(String),
    WebSocket(tungstenite::Error),
    NotImplemented(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Usage(message) => write!(f, "{message}"),
            AppError::Io(err) => write!(f, "{err}"),
            AppError::Json(err) => write!(f, "{err}"),
            AppError::Remote(message) => write!(f, "{message}"),
            AppError::WebSocket(err) => write!(f, "{err}"),
            AppError::NotImplemented(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::Io(err)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::Json(err)
    }
}

impl From<tungstenite::Error> for AppError {
    fn from(err: tungstenite::Error) -> Self {
        AppError::WebSocket(err)
    }
}
