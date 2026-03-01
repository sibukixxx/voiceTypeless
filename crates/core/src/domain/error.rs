use serde::Serialize;

/// アプリケーション共通エラーコード
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ErrorCode {
    #[serde(rename = "E_PERMISSION")]
    Permission,
    #[serde(rename = "E_DEVICE")]
    Device,
    #[serde(rename = "E_TIMEOUT")]
    Timeout,
    #[serde(rename = "E_STT_UNAVAILABLE")]
    SttUnavailable,
    #[serde(rename = "E_INVALID_STATE")]
    InvalidState,
    #[serde(rename = "E_INTERNAL")]
    Internal,
    #[serde(rename = "E_STORAGE")]
    Storage,
    #[serde(rename = "E_REWRITE")]
    Rewrite,
    #[serde(rename = "E_PIPELINE")]
    Pipeline,
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorCode::Permission => write!(f, "E_PERMISSION"),
            ErrorCode::Device => write!(f, "E_DEVICE"),
            ErrorCode::Timeout => write!(f, "E_TIMEOUT"),
            ErrorCode::SttUnavailable => write!(f, "E_STT_UNAVAILABLE"),
            ErrorCode::InvalidState => write!(f, "E_INVALID_STATE"),
            ErrorCode::Internal => write!(f, "E_INTERNAL"),
            ErrorCode::Storage => write!(f, "E_STORAGE"),
            ErrorCode::Rewrite => write!(f, "E_REWRITE"),
            ErrorCode::Pipeline => write!(f, "E_PIPELINE"),
        }
    }
}

/// アプリケーションエラー（イベントペイロード兼用）
#[derive(Debug, Clone, Serialize)]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
    pub recoverable: bool,
}

impl AppError {
    pub fn invalid_state(msg: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::InvalidState,
            message: msg.into(),
            recoverable: true,
        }
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::Internal,
            message: msg.into(),
            recoverable: false,
        }
    }

    pub fn device(msg: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::Device,
            message: msg.into(),
            recoverable: true,
        }
    }

    pub fn storage(msg: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::Storage,
            message: msg.into(),
            recoverable: false,
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:?}] {}", self.code, self.message)
    }
}

impl std::error::Error for AppError {}
