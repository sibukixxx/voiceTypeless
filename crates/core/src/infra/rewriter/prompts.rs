/// モード別リライトプロンプトテンプレート

/// Memo モード: フィラー除去 + 箇条書き化
pub const SYSTEM_MEMO: &str = "\
あなたは音声書き起こしテキストを整理するアシスタントです。
以下のルールに従って、テキストをリライトしてください：
- フィラー（えーと、あの、まあ、ええと等）を除去
- 要点を箇条書き（マークダウンリスト）で整理
- 重複する内容は統合
- 原文の意味を変えない
- 専門用語はそのまま保持";

/// Tech モード: 技術用語保持 + コードブロック維持
pub const SYSTEM_TECH: &str = "\
あなたは技術文書のリライトアシスタントです。
以下のルールに従って、音声書き起こしテキストをリライトしてください：
- プログラミング用語・技術用語はそのまま保持（英語のまま）
- コードやコマンドはコードブロック（```）で囲む
- フィラーや冗長な表現を除去
- 段落分けして読みやすく整形
- 変数名、関数名、クラス名は原文通りに保持";

/// Email JP モード: 丁寧語ビジネスメール
pub const SYSTEM_EMAIL_JP: &str = "\
あなたはビジネスメール作成アシスタントです。
音声書き起こしテキストを、以下のルールに従って丁寧な日本語ビジネスメールに変換してください：
- 敬語・丁寧語を使用
- ビジネスメールの定型フォーマット（挨拶→本文→締め）
- フィラーや口語表現を除去
- 箇条書きは適宜使用
- 要件を明確にまとめる";

/// Minutes モード: 決定事項 / TODO / 議論ポイント抽出
pub const SYSTEM_MINUTES: &str = "\
あなたは議事録作成アシスタントです。
音声書き起こしテキストから、以下の3カテゴリに分類して議事録を作成してください：

## 決定事項
- 確定した事項をリスト化

## TODO
- アクションアイテムをリスト化（担当者が分かれば記載）

## 議論ポイント
- 議論された内容の要約

フィラーや冗長な表現は除去し、簡潔にまとめてください。";

/// 辞書ヒントを含むユーザープロンプトを構築する
pub fn build_prompt(text: &str, dictionary_hints: &[String]) -> (String, String) {
    let mut user_msg = String::new();

    if !dictionary_hints.is_empty() {
        user_msg.push_str("【用語辞書（これらの用語は変更しないでください）】\n");
        for hint in dictionary_hints {
            user_msg.push_str("- ");
            user_msg.push_str(hint);
            user_msg.push('\n');
        }
        user_msg.push('\n');
    }

    user_msg.push_str("【書き起こしテキスト】\n");
    user_msg.push_str(text);

    (user_msg, String::new())
}

/// モードに対応するシステムプロンプトを取得する
pub fn system_prompt_for_mode(mode: &crate::domain::types::Mode) -> Option<&'static str> {
    match mode {
        crate::domain::types::Mode::Raw => None, // Raw はリライトスキップ
        crate::domain::types::Mode::Memo => Some(SYSTEM_MEMO),
        crate::domain::types::Mode::Tech => Some(SYSTEM_TECH),
        crate::domain::types::Mode::EmailJp => Some(SYSTEM_EMAIL_JP),
        crate::domain::types::Mode::Minutes => Some(SYSTEM_MINUTES),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::Mode;

    #[test]
    fn test_build_prompt_no_hints() {
        let (user, _) = build_prompt("テストテキスト", &[]);
        assert!(user.contains("テストテキスト"));
        assert!(!user.contains("用語辞書"));
    }

    #[test]
    fn test_build_prompt_with_hints() {
        let hints = vec!["Rust".to_string(), "Tauri".to_string()];
        let (user, _) = build_prompt("テストテキスト", &hints);
        assert!(user.contains("用語辞書"));
        assert!(user.contains("Rust"));
        assert!(user.contains("Tauri"));
    }

    #[test]
    fn test_system_prompt_for_mode() {
        assert!(system_prompt_for_mode(&Mode::Raw).is_none());
        assert!(system_prompt_for_mode(&Mode::Memo).is_some());
        assert!(system_prompt_for_mode(&Mode::Tech).is_some());
        assert!(system_prompt_for_mode(&Mode::EmailJp).is_some());
        assert!(system_prompt_for_mode(&Mode::Minutes).is_some());
    }
}
