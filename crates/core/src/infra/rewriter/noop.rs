use async_trait::async_trait;
use super::{RewriteContext, RewriteError, Rewriter};

/// NoopRewriter: テキストをそのまま返すモック実装。
/// Phase3でLLM連携のリライターに差し替える。
pub struct NoopRewriter;

#[async_trait]
impl Rewriter for NoopRewriter {
    async fn rewrite(
        &self,
        text: &str,
        _ctx: RewriteContext,
    ) -> Result<String, RewriteError> {
        // モック: プレフィックスを付けてそのまま返す
        Ok(format!("[rewritten] {text}"))
    }

    fn name(&self) -> &str {
        "noop"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::Mode;

    #[tokio::test]
    async fn test_noop_rewriter() {
        let rewriter = NoopRewriter;
        let ctx = RewriteContext {
            mode: Mode::Memo,
            dictionary_hints: vec![],
        };
        let result = rewriter.rewrite("テストテキスト", ctx).await.unwrap();
        assert_eq!(result, "[rewritten] テストテキスト");
    }

    #[test]
    fn test_noop_name() {
        assert_eq!(NoopRewriter.name(), "noop");
    }
}
