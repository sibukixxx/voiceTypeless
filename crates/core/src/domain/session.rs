use serde::Serialize;

use super::error::AppError;
use super::types::{DeliverPolicy, Mode};

/// セッション状態
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    Idle,
    Recording,
    Transcribing,
    Rewriting,
    Delivering,
    Error {
        code: String,
        message: String,
        recoverable: bool,
    },
}

impl SessionState {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Idle => "idle",
            Self::Recording => "recording",
            Self::Transcribing => "transcribing",
            Self::Rewriting => "rewriting",
            Self::Delivering => "delivering",
            Self::Error { .. } => "error",
        }
    }
}

/// セッション
#[derive(Debug, Clone)]
pub struct Session {
    pub session_id: String,
    pub state: SessionState,
    pub mode: Mode,
    pub deliver_policy: DeliverPolicy,
    pub created_at: String,
    pub updated_at: String,
}

impl Session {
    pub fn new(session_id: String, mode: Mode, deliver_policy: DeliverPolicy, now: String) -> Self {
        Self {
            session_id,
            state: SessionState::Idle,
            mode,
            deliver_policy,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

/// セッションマネージャー（単一アクティブセッション）
pub struct SessionManager {
    active: Option<Session>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self { active: None }
    }

    /// アクティブセッションの参照
    pub fn active(&self) -> Option<&Session> {
        self.active.as_ref()
    }

    /// アクティブセッションのmutable参照
    pub fn active_mut(&mut self) -> Option<&mut Session> {
        self.active.as_mut()
    }

    /// 新しいセッションを開始（既存があれば自動停止）
    pub fn start_session(
        &mut self,
        session_id: String,
        mode: Mode,
        deliver_policy: DeliverPolicy,
        now: String,
    ) -> Result<&Session, AppError> {
        // 既存セッションは自動停止
        self.active = Some(Session::new(session_id, mode, deliver_policy, now));
        Ok(self.active.as_ref().unwrap())
    }

    /// セッション停止
    pub fn stop_session(&mut self) -> Result<Option<Session>, AppError> {
        let session = self.active.take();
        Ok(session)
    }

    /// pause_recording: Recording→Idle（パイプラインモード用）
    pub fn pause_recording(&mut self, now: String) -> Result<StateTransition, AppError> {
        let session = self
            .active
            .as_mut()
            .ok_or_else(|| AppError::internal("アクティブセッションがありません"))?;

        let prev = session.state.as_str().to_string();

        match &session.state {
            SessionState::Recording => {
                session.state = SessionState::Idle;
                session.updated_at = now;
                Ok(StateTransition {
                    session_id: session.session_id.clone(),
                    prev_state: prev,
                    new_state: session.state.clone(),
                })
            }
            other => Err(AppError::invalid_state(format!(
                "pause_recording は {} 状態では実行できません",
                other.as_str()
            ))),
        }
    }

    /// toggle_recording: Idle→Recording, Recording→Transcribing
    pub fn toggle_recording(&mut self, now: String) -> Result<StateTransition, AppError> {
        let session = self
            .active
            .as_mut()
            .ok_or_else(|| AppError::internal("アクティブセッションがありません"))?;

        let prev = session.state.as_str().to_string();

        match &session.state {
            SessionState::Idle => {
                session.state = SessionState::Recording;
                session.updated_at = now;
                Ok(StateTransition {
                    session_id: session.session_id.clone(),
                    prev_state: prev,
                    new_state: session.state.clone(),
                })
            }
            SessionState::Recording => {
                session.state = SessionState::Transcribing;
                session.updated_at = now;
                Ok(StateTransition {
                    session_id: session.session_id.clone(),
                    prev_state: prev,
                    new_state: session.state.clone(),
                })
            }
            other => Err(AppError::invalid_state(format!(
                "toggle_recording は {} 状態では実行できません",
                other.as_str()
            ))),
        }
    }

    /// STT完了: Transcribing→Rewriting (mode≠raw) or Transcribing→Delivering (mode=raw)
    pub fn on_transcript_done(&mut self, now: String) -> Result<StateTransition, AppError> {
        let session = self
            .active
            .as_mut()
            .ok_or_else(|| AppError::internal("アクティブセッションがありません"))?;

        let prev = session.state.as_str().to_string();

        match &session.state {
            SessionState::Transcribing => {
                if session.mode == Mode::Raw {
                    session.state = SessionState::Delivering;
                } else {
                    session.state = SessionState::Rewriting;
                }
                session.updated_at = now;
                Ok(StateTransition {
                    session_id: session.session_id.clone(),
                    prev_state: prev,
                    new_state: session.state.clone(),
                })
            }
            other => Err(AppError::invalid_state(format!(
                "on_transcript_done は {} 状態では実行できません",
                other.as_str()
            ))),
        }
    }

    /// リライト完了: Rewriting→Delivering
    pub fn on_rewrite_done(&mut self, now: String) -> Result<StateTransition, AppError> {
        let session = self
            .active
            .as_mut()
            .ok_or_else(|| AppError::internal("アクティブセッションがありません"))?;

        let prev = session.state.as_str().to_string();

        match &session.state {
            SessionState::Rewriting => {
                session.state = SessionState::Delivering;
                session.updated_at = now;
                Ok(StateTransition {
                    session_id: session.session_id.clone(),
                    prev_state: prev,
                    new_state: session.state.clone(),
                })
            }
            other => Err(AppError::invalid_state(format!(
                "on_rewrite_done は {} 状態では実行できません",
                other.as_str()
            ))),
        }
    }

    /// 配信完了: Delivering→Idle
    pub fn on_deliver_done(&mut self, now: String) -> Result<StateTransition, AppError> {
        let session = self
            .active
            .as_mut()
            .ok_or_else(|| AppError::internal("アクティブセッションがありません"))?;

        let prev = session.state.as_str().to_string();

        match &session.state {
            SessionState::Delivering => {
                session.state = SessionState::Idle;
                session.updated_at = now;
                Ok(StateTransition {
                    session_id: session.session_id.clone(),
                    prev_state: prev,
                    new_state: session.state.clone(),
                })
            }
            other => Err(AppError::invalid_state(format!(
                "on_deliver_done は {} 状態では実行できません",
                other.as_str()
            ))),
        }
    }

    /// エラー状態に遷移
    pub fn on_error(
        &mut self,
        code: String,
        message: String,
        recoverable: bool,
        now: String,
    ) -> Option<StateTransition> {
        let session = self.active.as_mut()?;
        let prev = session.state.as_str().to_string();
        session.state = SessionState::Error {
            code,
            message,
            recoverable,
        };
        session.updated_at = now;
        Some(StateTransition {
            session_id: session.session_id.clone(),
            prev_state: prev,
            new_state: session.state.clone(),
        })
    }

    /// エラーからの復帰（recoverable の場合のみ）
    pub fn recover_from_error(&mut self, now: String) -> Result<StateTransition, AppError> {
        let session = self
            .active
            .as_mut()
            .ok_or_else(|| AppError::internal("アクティブセッションがありません"))?;

        let prev = session.state.as_str().to_string();

        match &session.state {
            SessionState::Error { recoverable, .. } if *recoverable => {
                session.state = SessionState::Idle;
                session.updated_at = now;
                Ok(StateTransition {
                    session_id: session.session_id.clone(),
                    prev_state: prev,
                    new_state: session.state.clone(),
                })
            }
            SessionState::Error { .. } => {
                Err(AppError::invalid_state("回復不可能なエラーです"))
            }
            other => Err(AppError::invalid_state(format!(
                "recover_from_error は {} 状態では実行できません",
                other.as_str()
            ))),
        }
    }

    /// モード変更
    pub fn set_mode(&mut self, mode: Mode) -> Result<(), AppError> {
        let session = self
            .active
            .as_mut()
            .ok_or_else(|| AppError::internal("アクティブセッションがありません"))?;
        session.mode = mode;
        Ok(())
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 状態遷移イベントペイロード
#[derive(Debug, Clone, Serialize)]
pub struct StateTransition {
    pub session_id: String,
    pub prev_state: String,
    pub new_state: SessionState,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::{DeliverPolicy, Mode};

    fn now() -> String {
        "2025-01-15T10:30:00Z".to_string()
    }

    fn setup_manager() -> SessionManager {
        let mut mgr = SessionManager::new();
        mgr.start_session(
            "test-session".to_string(),
            Mode::Memo,
            DeliverPolicy::Clipboard,
            now(),
        )
        .unwrap();
        mgr
    }

    #[test]
    fn test_idle_to_recording() {
        let mut mgr = setup_manager();
        let t = mgr.toggle_recording(now()).unwrap();
        assert_eq!(t.prev_state, "idle");
        assert_eq!(t.new_state, SessionState::Recording);
    }

    #[test]
    fn test_recording_to_transcribing() {
        let mut mgr = setup_manager();
        mgr.toggle_recording(now()).unwrap();
        let t = mgr.toggle_recording(now()).unwrap();
        assert_eq!(t.prev_state, "recording");
        assert_eq!(t.new_state, SessionState::Transcribing);
    }

    #[test]
    fn test_transcribing_to_rewriting_when_not_raw() {
        let mut mgr = setup_manager();
        mgr.toggle_recording(now()).unwrap();
        mgr.toggle_recording(now()).unwrap();
        let t = mgr.on_transcript_done(now()).unwrap();
        assert_eq!(t.new_state, SessionState::Rewriting);
    }

    #[test]
    fn test_transcribing_to_delivering_when_raw() {
        let mut mgr = SessionManager::new();
        mgr.start_session(
            "raw-session".to_string(),
            Mode::Raw,
            DeliverPolicy::Clipboard,
            now(),
        )
        .unwrap();
        mgr.toggle_recording(now()).unwrap();
        mgr.toggle_recording(now()).unwrap();
        let t = mgr.on_transcript_done(now()).unwrap();
        assert_eq!(t.new_state, SessionState::Delivering);
    }

    #[test]
    fn test_rewriting_to_delivering() {
        let mut mgr = setup_manager();
        mgr.toggle_recording(now()).unwrap();
        mgr.toggle_recording(now()).unwrap();
        mgr.on_transcript_done(now()).unwrap();
        let t = mgr.on_rewrite_done(now()).unwrap();
        assert_eq!(t.new_state, SessionState::Delivering);
    }

    #[test]
    fn test_delivering_to_idle() {
        let mut mgr = setup_manager();
        mgr.toggle_recording(now()).unwrap();
        mgr.toggle_recording(now()).unwrap();
        mgr.on_transcript_done(now()).unwrap();
        mgr.on_rewrite_done(now()).unwrap();
        let t = mgr.on_deliver_done(now()).unwrap();
        assert_eq!(t.new_state, SessionState::Idle);
    }

    #[test]
    fn test_full_cycle() {
        let mut mgr = setup_manager();
        // Idle → Recording
        mgr.toggle_recording(now()).unwrap();
        // Recording → Transcribing
        mgr.toggle_recording(now()).unwrap();
        // Transcribing → Rewriting (mode=memo)
        mgr.on_transcript_done(now()).unwrap();
        // Rewriting → Delivering
        mgr.on_rewrite_done(now()).unwrap();
        // Delivering → Idle
        mgr.on_deliver_done(now()).unwrap();
        assert_eq!(mgr.active().unwrap().state, SessionState::Idle);
    }

    #[test]
    fn test_invalid_toggle_in_transcribing() {
        let mut mgr = setup_manager();
        mgr.toggle_recording(now()).unwrap();
        mgr.toggle_recording(now()).unwrap();
        let result = mgr.toggle_recording(now());
        assert!(result.is_err());
    }

    #[test]
    fn test_error_and_recovery() {
        let mut mgr = setup_manager();
        mgr.toggle_recording(now()).unwrap();
        mgr.on_error(
            "E_DEVICE".to_string(),
            "マイクが切断されました".to_string(),
            true,
            now(),
        );
        let t = mgr.recover_from_error(now()).unwrap();
        assert_eq!(t.new_state, SessionState::Idle);
    }

    #[test]
    fn test_non_recoverable_error() {
        let mut mgr = setup_manager();
        mgr.on_error(
            "E_INTERNAL".to_string(),
            "致命的エラー".to_string(),
            false,
            now(),
        );
        let result = mgr.recover_from_error(now());
        assert!(result.is_err());
    }

    #[test]
    fn test_stop_session() {
        let mut mgr = setup_manager();
        let session = mgr.stop_session().unwrap();
        assert!(session.is_some());
        assert!(mgr.active().is_none());
    }

    #[test]
    fn test_set_mode() {
        let mut mgr = setup_manager();
        mgr.set_mode(Mode::Tech).unwrap();
        assert_eq!(mgr.active().unwrap().mode, Mode::Tech);
    }
}
