mod clipboard;

pub use clipboard::ClipboardOutput;

use crate::domain::error::AppError;
use crate::domain::types::DeliverTarget;

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

    /// DeliverTarget に従って配信する
    pub fn deliver(&self, target: DeliverTarget, text: &str) -> Result<(), AppError> {
        match target {
            DeliverTarget::Clipboard => self.deliver_clipboard(text),
            DeliverTarget::Paste => Err(AppError::invalid_state("paste target は未実装です")),
            DeliverTarget::FileAppend => {
                Err(AppError::invalid_state("file_append target は未実装です"))
            }
            DeliverTarget::Webhook => Err(AppError::invalid_state("webhook target は未実装です")),
        }
    }
}

impl Default for OutputRouter {
    fn default() -> Self {
        Self::new()
    }
}
