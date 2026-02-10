#[cfg(test)]
mod tests {
    use crate::domain::error::{AppError, ErrorCode};
    use crate::domain::session::{SessionState, StateTransition};
    use crate::domain::types::{DeliverPolicy, DictionaryEntry, DictionaryScope, Mode};

    #[test]
    fn test_mode_serialization() {
        assert_eq!(serde_json::to_string(&Mode::Raw).unwrap(), "\"raw\"");
        assert_eq!(serde_json::to_string(&Mode::Memo).unwrap(), "\"memo\"");
        assert_eq!(serde_json::to_string(&Mode::Tech).unwrap(), "\"tech\"");
        assert_eq!(
            serde_json::to_string(&Mode::EmailJp).unwrap(),
            "\"email_jp\""
        );
        assert_eq!(
            serde_json::to_string(&Mode::Minutes).unwrap(),
            "\"minutes\""
        );
    }

    #[test]
    fn test_mode_deserialization() {
        assert_eq!(
            serde_json::from_str::<Mode>("\"raw\"").unwrap(),
            Mode::Raw
        );
        assert_eq!(
            serde_json::from_str::<Mode>("\"email_jp\"").unwrap(),
            Mode::EmailJp
        );
    }

    #[test]
    fn test_deliver_policy_serialization() {
        let policy = DeliverPolicy::Clipboard;
        let json = serde_json::to_string(&policy).unwrap();
        assert!(json.contains("clipboard"));
    }

    #[test]
    fn test_session_state_serialization() {
        assert_eq!(
            serde_json::to_string(&SessionState::Idle).unwrap(),
            "\"idle\""
        );
        assert_eq!(
            serde_json::to_string(&SessionState::Recording).unwrap(),
            "\"recording\""
        );

        let error_state = SessionState::Error {
            code: "E_DEVICE".to_string(),
            message: "test".to_string(),
            recoverable: true,
        };
        let json = serde_json::to_string(&error_state).unwrap();
        assert!(json.contains("E_DEVICE"));
        assert!(json.contains("recoverable"));
    }

    #[test]
    fn test_error_code_serialization() {
        assert_eq!(
            serde_json::to_string(&ErrorCode::Permission).unwrap(),
            "\"E_PERMISSION\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::InvalidState).unwrap(),
            "\"E_INVALID_STATE\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::SttUnavailable).unwrap(),
            "\"E_STT_UNAVAILABLE\""
        );
    }

    #[test]
    fn test_app_error_serialization() {
        let err = AppError::invalid_state("テスト");
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("E_INVALID_STATE"));
        assert!(json.contains("recoverable"));
    }

    #[test]
    fn test_state_transition_serialization() {
        let t = StateTransition {
            session_id: "test-id".to_string(),
            prev_state: "idle".to_string(),
            new_state: SessionState::Recording,
        };
        let json = serde_json::to_string(&t).unwrap();
        assert!(json.contains("test-id"));
        assert!(json.contains("idle"));
        assert!(json.contains("recording"));
    }

    #[test]
    fn test_dictionary_entry_roundtrip() {
        let entry = DictionaryEntry {
            id: Some("d1".to_string()),
            scope: DictionaryScope::Global,
            mode: Some(Mode::Tech),
            pattern: "test".to_string(),
            replacement: "テスト".to_string(),
            priority: 10,
            enabled: true,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let roundtrip: DictionaryEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip.id, entry.id);
        assert_eq!(roundtrip.scope, entry.scope);
        assert_eq!(roundtrip.mode, entry.mode);
        assert_eq!(roundtrip.pattern, entry.pattern);
        assert_eq!(roundtrip.replacement, entry.replacement);
        assert_eq!(roundtrip.priority, entry.priority);
        assert_eq!(roundtrip.enabled, entry.enabled);
    }
}
