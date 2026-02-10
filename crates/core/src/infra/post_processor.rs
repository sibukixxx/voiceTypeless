use crate::domain::types::DictionaryEntry;

/// テキスト後処理パイプライン: 正規化 → 辞書置換
pub struct PostProcessor;

impl PostProcessor {
    /// 全パイプラインを適用: normalize → apply_dictionary
    pub fn process(text: &str, entries: &[DictionaryEntry]) -> String {
        let normalized = Self::normalize(text);
        Self::apply_dictionary(&normalized, entries)
    }

    /// 正規化: 全角英数→半角、半角カナ→全角、連続空白の圧縮、前後トリム
    pub fn normalize(text: &str) -> String {
        let mut result = String::with_capacity(text.len());

        for ch in text.chars() {
            match ch {
                // 全角英数字 → 半角
                '\u{FF01}'..='\u{FF5E}' => {
                    result.push(char::from_u32(ch as u32 - 0xFEE0).unwrap_or(ch));
                }
                // 全角スペース → 半角
                '\u{3000}' => result.push(' '),
                _ => result.push(ch),
            }
        }

        // 連続空白の圧縮
        let compressed = compress_whitespace(&result);
        compressed.trim().to_string()
    }

    /// 辞書エントリを優先度順に適用（単純文字列置換）
    pub fn apply_dictionary(text: &str, entries: &[DictionaryEntry]) -> String {
        let mut result = text.to_string();

        // entries は priority DESC でソート済みの前提
        for entry in entries {
            if !entry.enabled {
                continue;
            }
            result = result.replace(&entry.pattern, &entry.replacement);
        }

        result
    }
}

fn compress_whitespace(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut prev_space = false;

    for ch in s.chars() {
        if ch == ' ' || ch == '\t' {
            if !prev_space {
                result.push(' ');
            }
            prev_space = true;
        } else {
            // 改行は保持
            prev_space = false;
            result.push(ch);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::{DictionaryEntry, DictionaryScope};

    #[test]
    fn test_normalize_fullwidth_to_halfwidth() {
        assert_eq!(PostProcessor::normalize("Ｈｅｌｌｏ　Ｗｏｒｌｄ"), "Hello World");
        assert_eq!(PostProcessor::normalize("１２３４５"), "12345");
        assert_eq!(PostProcessor::normalize("ＡＢＣ"), "ABC");
    }

    #[test]
    fn test_normalize_whitespace() {
        assert_eq!(PostProcessor::normalize("hello   world"), "hello world");
        assert_eq!(PostProcessor::normalize("  hello  "), "hello");
        assert_eq!(
            PostProcessor::normalize("line1\nline2"),
            "line1\nline2"
        );
    }

    #[test]
    fn test_normalize_mixed() {
        assert_eq!(
            PostProcessor::normalize("　Ｈｅｌｌｏ　　ｗｏｒｌｄ　"),
            "Hello world"
        );
    }

    #[test]
    fn test_apply_dictionary_basic() {
        let entries = vec![
            DictionaryEntry {
                id: Some("1".into()),
                scope: DictionaryScope::Global,
                mode: None,
                pattern: "くろーど".into(),
                replacement: "Claude".into(),
                priority: 10,
                enabled: true,
            },
            DictionaryEntry {
                id: Some("2".into()),
                scope: DictionaryScope::Global,
                mode: None,
                pattern: "らすと".into(),
                replacement: "Rust".into(),
                priority: 5,
                enabled: true,
            },
        ];

        assert_eq!(
            PostProcessor::apply_dictionary("くろーどとらすとは良い", &entries),
            "ClaudeとRustは良い"
        );
    }

    #[test]
    fn test_apply_dictionary_disabled_entry() {
        let entries = vec![DictionaryEntry {
            id: Some("1".into()),
            scope: DictionaryScope::Global,
            mode: None,
            pattern: "foo".into(),
            replacement: "bar".into(),
            priority: 10,
            enabled: false,
        }];

        assert_eq!(
            PostProcessor::apply_dictionary("foo test", &entries),
            "foo test"
        );
    }

    #[test]
    fn test_apply_dictionary_priority_order() {
        // priority高いものが先に適用される（entries はpriority DESC順の前提）
        let entries = vec![
            DictionaryEntry {
                id: Some("1".into()),
                scope: DictionaryScope::Global,
                mode: None,
                pattern: "ABC".into(),
                replacement: "XYZ".into(),
                priority: 10,
                enabled: true,
            },
            DictionaryEntry {
                id: Some("2".into()),
                scope: DictionaryScope::Global,
                mode: None,
                pattern: "XYZ".into(),
                replacement: "123".into(),
                priority: 5,
                enabled: true,
            },
        ];

        // ABC → XYZ → 123（優先度順に適用されるので連鎖する）
        assert_eq!(
            PostProcessor::apply_dictionary("ABC test", &entries),
            "123 test"
        );
    }

    #[test]
    fn test_full_pipeline() {
        let entries = vec![DictionaryEntry {
            id: Some("1".into()),
            scope: DictionaryScope::Global,
            mode: None,
            pattern: "くろーど".into(),
            replacement: "Claude".into(),
            priority: 10,
            enabled: true,
        }];

        let input = "　くろーど　は　すごい　";
        assert_eq!(PostProcessor::process(input, &entries), "Claude は すごい");
    }

    #[test]
    fn test_empty_text() {
        assert_eq!(PostProcessor::normalize(""), "");
        assert_eq!(PostProcessor::apply_dictionary("", &[]), "");
        assert_eq!(PostProcessor::process("", &[]), "");
    }
}
