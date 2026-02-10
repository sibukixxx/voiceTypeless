use crate::domain::error::AppError;
use super::OutputTarget;

/// クリップボード出力
pub struct ClipboardOutput {
    // Phase2: arboard を使う。テスト時はモックに切り替え可能。
}

impl ClipboardOutput {
    pub fn new() -> Self {
        Self {}
    }
}

impl OutputTarget for ClipboardOutput {
    fn deliver(&self, text: &str) -> Result<(), AppError> {
        let mut ctx = arboard::Clipboard::new()
            .map_err(|e| AppError::internal(format!("クリップボード初期化失敗: {e}")))?;
        ctx.set_text(text)
            .map_err(|e| AppError::internal(format!("クリップボード書き込み失敗: {e}")))?;
        log::info!("クリップボードに出力: {} 文字", text.len());
        Ok(())
    }

    fn name(&self) -> &str {
        "clipboard"
    }
}
