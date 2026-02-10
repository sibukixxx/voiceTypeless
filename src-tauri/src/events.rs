use serde::Serialize;
use tauri::{AppHandle, Emitter};

/// イベント名定数
pub const SESSION_STATE_CHANGED: &str = "session_state_changed";
pub const TRANSCRIPT_PARTIAL: &str = "transcript_partial";
pub const TRANSCRIPT_FINAL: &str = "transcript_final";
pub const REWRITE_DONE: &str = "rewrite_done";
pub const DELIVER_DONE: &str = "deliver_done";
pub const ERROR: &str = "error";

/// 統一イベント送信関数
pub fn emit_event<S: Serialize + Clone>(
    app: &AppHandle,
    event_name: &str,
    payload: S,
) {
    if let Err(e) = app.emit(event_name, payload) {
        log::error!("イベント送信失敗 [{event_name}]: {e}");
    }
}

/// session_state_changed ペイロード
#[derive(Debug, Clone, Serialize)]
pub struct SessionStateChangedPayload {
    pub session_id: String,
    pub prev_state: String,
    pub new_state: String,
    pub timestamp: String,
}

/// error ペイロード
#[derive(Debug, Clone, Serialize)]
pub struct ErrorPayload {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}
