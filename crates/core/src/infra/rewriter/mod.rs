pub mod claude;
mod noop;
pub mod prompts;

pub use noop::NoopRewriter;

use async_trait::async_trait;
use crate::domain::types::Mode;

/// リライトエラー
#[derive(Debug, thiserror::Error)]
pub enum RewriteError {
    #[error("Rewriter not available: {0}")]
    NotAvailable(String),
    #[error("Rewrite failed: {0}")]
    Failed(String),
    #[error("Rewrite timeout")]
    Timeout,
}

/// リライトコンテキスト
#[derive(Debug, Clone)]
pub struct RewriteContext {
    pub mode: Mode,
    pub dictionary_hints: Vec<String>,
}

/// リライター trait（Agent Bや外部LLMが実装する）
#[async_trait]
pub trait Rewriter: Send + Sync {
    async fn rewrite(
        &self,
        text: &str,
        ctx: RewriteContext,
    ) -> Result<String, RewriteError>;

    fn name(&self) -> &str;
}
