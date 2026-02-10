mod clipboard;

pub use clipboard::ClipboardOutput;

use crate::domain::error::AppError;

/// 出力先 trait
pub trait OutputTarget: Send + Sync {
    fn deliver(&self, text: &str) -> Result<(), AppError>;
    fn name(&self) -> &str;
}

/// 出力ルーター: DeliverPolicy に基づいてテキストを配信
pub struct OutputRouter {
    clipboard: ClipboardOutput,
}

impl OutputRouter {
    pub fn new() -> Self {
        Self {
            clipboard: ClipboardOutput::new(),
        }
    }

    /// テキストをクリップボードに出力（Phase2はclipboardのみ）
    pub fn deliver_clipboard(&self, text: &str) -> Result<(), AppError> {
        self.clipboard.deliver(text)
    }
}

impl Default for OutputRouter {
    fn default() -> Self {
        Self::new()
    }
}
