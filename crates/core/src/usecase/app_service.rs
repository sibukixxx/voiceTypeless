use std::sync::Mutex;

use crate::domain::error::AppError;
use crate::domain::session::{SessionManager, SessionState, StateTransition};
use crate::domain::types::{
    DeliverPolicy, DictionaryEntry, HistoryPage, Mode, SessionDetail,
};
use crate::infra::storage::Storage;

/// アプリケーションサービス（Tauri State として管理される）
pub struct AppService {
    session_mgr: Mutex<SessionManager>,
    storage: Mutex<Storage>,
}

impl AppService {
    pub fn new(storage: Storage) -> Self {
        Self {
            session_mgr: Mutex::new(SessionManager::new()),
            storage: Mutex::new(storage),
        }
    }

    /// セッション開始
    pub fn start_session(
        &self,
        mode: Mode,
        deliver_policy: DeliverPolicy,
    ) -> Result<(String, StateTransition), AppError> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        let mut mgr = self.session_mgr.lock().unwrap();
        mgr.start_session(session_id.clone(), mode, deliver_policy, now.clone())?;

        let storage = self.storage.lock().unwrap();
        storage.insert_session(&session_id, mode, &now)?;

        let transition = StateTransition {
            session_id: session_id.clone(),
            prev_state: "none".to_string(),
            new_state: SessionState::Idle,
        };

        Ok((session_id, transition))
    }

    /// セッション停止
    pub fn stop_session(&self) -> Result<Option<StateTransition>, AppError> {
        let mut mgr = self.session_mgr.lock().unwrap();
        let session = mgr.stop_session()?;

        if let Some(ref s) = session {
            let now = chrono::Utc::now().to_rfc3339();
            let storage = self.storage.lock().unwrap();
            storage.update_session_state(&s.session_id, "idle", &now)?;

            return Ok(Some(StateTransition {
                session_id: s.session_id.clone(),
                prev_state: s.state.as_str().to_string(),
                new_state: SessionState::Idle,
            }));
        }

        Ok(None)
    }

    /// 録音トグル
    pub fn toggle_recording(&self) -> Result<StateTransition, AppError> {
        let now = chrono::Utc::now().to_rfc3339();
        let mut mgr = self.session_mgr.lock().unwrap();
        let transition = mgr.toggle_recording(now.clone())?;

        let storage = self.storage.lock().unwrap();
        storage.update_session_state(
            &transition.session_id,
            transition.new_state.as_str(),
            &now,
        )?;

        // Recording→Transcribing の場合、セグメントを作成
        if transition.new_state == SessionState::Transcribing {
            let segment_id = uuid::Uuid::new_v4().to_string();
            storage.insert_segment(&segment_id, &transition.session_id, &now)?;
        }

        Ok(transition)
    }

    /// モード変更
    pub fn set_mode(&self, mode: Mode) -> Result<(), AppError> {
        let mut mgr = self.session_mgr.lock().unwrap();
        mgr.set_mode(mode)
    }

    /// STT完了通知（内部から呼ばれる）
    pub fn on_transcript_done(
        &self,
        segment_id: &str,
        text: &str,
        confidence: f32,
    ) -> Result<StateTransition, AppError> {
        let now = chrono::Utc::now().to_rfc3339();

        let storage = self.storage.lock().unwrap();
        storage.update_segment_text(segment_id, text, confidence)?;

        let mut mgr = self.session_mgr.lock().unwrap();
        let transition = mgr.on_transcript_done(now.clone())?;
        storage.update_session_state(
            &transition.session_id,
            transition.new_state.as_str(),
            &now,
        )?;

        Ok(transition)
    }

    /// リライト完了通知（内部から呼ばれる）
    pub fn on_rewrite_done(
        &self,
        segment_id: &str,
        rewritten_text: &str,
    ) -> Result<StateTransition, AppError> {
        let now = chrono::Utc::now().to_rfc3339();

        let storage = self.storage.lock().unwrap();
        storage.update_segment_rewritten(segment_id, rewritten_text)?;

        let mut mgr = self.session_mgr.lock().unwrap();
        let transition = mgr.on_rewrite_done(now.clone())?;
        storage.update_session_state(
            &transition.session_id,
            transition.new_state.as_str(),
            &now,
        )?;

        Ok(transition)
    }

    /// 配信完了通知（内部から呼ばれる）
    pub fn on_deliver_done(&self) -> Result<StateTransition, AppError> {
        let now = chrono::Utc::now().to_rfc3339();
        let mut mgr = self.session_mgr.lock().unwrap();
        let transition = mgr.on_deliver_done(now.clone())?;

        let storage = self.storage.lock().unwrap();
        storage.update_session_state(
            &transition.session_id,
            transition.new_state.as_str(),
            &now,
        )?;

        Ok(transition)
    }

    /// 履歴取得
    pub fn get_history(
        &self,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<HistoryPage, AppError> {
        let storage = self.storage.lock().unwrap();
        storage.list_history(limit, cursor)
    }

    /// セッション詳細取得
    pub fn get_session(&self, session_id: &str) -> Result<Option<SessionDetail>, AppError> {
        let storage = self.storage.lock().unwrap();
        storage.get_session_detail(session_id)
    }

    /// 辞書アップサート（スタブ）
    pub fn upsert_dictionary(&self, _entry: DictionaryEntry) -> Result<String, AppError> {
        // Phase2で実装。今はIDを返すだけ。
        Ok(uuid::Uuid::new_v4().to_string())
    }

    /// 辞書一覧（スタブ）
    pub fn list_dictionary(&self, _scope: Option<&str>) -> Result<Vec<DictionaryEntry>, AppError> {
        // Phase2で実装。
        Ok(vec![])
    }

    /// 現在のセッション状態を取得
    pub fn current_state(&self) -> Option<String> {
        let mgr = self.session_mgr.lock().unwrap();
        mgr.active().map(|s| s.state.as_str().to_string())
    }

    /// 現在のセッションIDを取得
    pub fn current_session_id(&self) -> Option<String> {
        let mgr = self.session_mgr.lock().unwrap();
        mgr.active().map(|s| s.session_id.clone())
    }
}
