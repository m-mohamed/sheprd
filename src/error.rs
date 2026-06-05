use thiserror::Error;

#[derive(Debug, Error)]
pub enum SheprdError {
    #[error("{0}")]
    Message(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("toml error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("home directory is unavailable")]
    MissingHome,
}

impl SheprdError {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Message(_) => "message",
            Self::Io(_) => "io",
            Self::Json(_) => "json",
            Self::Toml(_) => "toml",
            Self::MissingHome => "missing_home",
        }
    }

    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Message(_) => 2,
            Self::MissingHome => 2,
            Self::Io(_) | Self::Json(_) | Self::Toml(_) => 1,
        }
    }
}

pub type Result<T> = std::result::Result<T, SheprdError>;
