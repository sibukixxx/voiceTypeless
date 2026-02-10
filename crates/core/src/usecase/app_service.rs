use std::sync::Mutex;

use crate::domain::error::AppError;
use crate::domain::session::{SessionManager, SessionState, StateTransition};
use crate::domain::types::{
    DeliverPolicy, DictionaryEntry, HistoryPage, Mode, SessionDetail,
};
use crate::infra::output::OutputRouter;
use crate::infra::post_processor::PostProcessor;
use crate::infra::storage::Storage;

/// アプリケーションサービス（Tauri State として管理される）
pub struct AppService {
    session_mgr: Mutex<SessionManager>,
    storage: Mutex<Storage>,
    output_router: OutputRouter,
}

impl AppService {
    pub fn new(storage: Storage) -> Self {
        Self {
            session_mgr: Mutex::new(SessionManager::new()),
            storage: Mutex::new(storage),
            output_router: OutputRouter::new(),
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

    /// STT完了通知 + 後処理パイプライン適用
    pub fn on_transcript_done(
        &self,
        segment_id: &str,
        text: &str,
        confidence: f32,
    ) -> Result<(StateTransition, String), AppError> {
        let now = chrono::Utc::now().to_rfc3339();

        // 辞書エントリ取得
        let storage = self.storage.lock().unwrap();
        let mode_str = {
            let mgr = self.session_mgr.lock().unwrap();
            mgr.active()
                .map(|s| {
                    serde_json::to_value(s.mode)
                        .ok()
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                })
                .flatten()
        };
        let dict_entries = storage
            .get_enabled_dictionary_entries("global", mode_str.as_deref())
            .unwrap_or_default();

        // 後処理パイプライン適用
        let processed_text = PostProcessor::process(text, &dict_entries);

        storage.update_segment_text(segment_id, &processed_text, confidence)?;

        let mut mgr = self.session_mgr.lock().unwrap();
        let transition = mgr.on_transcript_done(now.clone())?;
        storage.update_session_state(
            &transition.session_id,
            transition.new_state.as_str(),
            &now,
        )?;

        Ok((transition, processed_text))
    }

    /// リライト完了通知
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

    /// 配信実行 + 状態遷移
    pub fn deliver(&self, text: &str) -> Result<StateTransition, AppError> {
        // クリップボードに出力
        self.output_router.deliver_clipboard(text)?;

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

    /// deliver_last: 最後のセグメントのテキストをクリップボードに出力
    pub fn deliver_last(&self) -> Result<(StateTransition, String), AppError> {
        let session_id = {
            let mgr = self.session_mgr.lock().unwrap();
            mgr.active()
                .map(|s| s.session_id.clone())
                .ok_or_else(|| AppError::internal("アクティブセッションがありません"))?
        };

        let storage = self.storage.lock().unwrap();
        let detail = storage
            .get_session_detail(&session_id)?
            .ok_or_else(|| AppError::internal("セッション詳細が見つかりません"))?;
        drop(storage);

        let last_segment = detail
            .segments
            .last()
            .ok_or_else(|| AppError::internal("セグメントがありません"))?;

        let text = last_segment
            .rewritten_text
            .as_deref()
            .unwrap_or(&last_segment.raw_text);

        self.output_router.deliver_clipboard(text)?;

        // Delivering状態でなければ状態遷移はスキップ（手動deliver）
        let mgr = self.session_mgr.lock().unwrap();
        let current_state = mgr.active().map(|s| s.state.as_str().to_string());
        drop(mgr);

        let text_result = text.to_string();

        if current_state.as_deref() == Some("delivering") {
            let now = chrono::Utc::now().to_rfc3339();
            let mut mgr = self.session_mgr.lock().unwrap();
            let transition = mgr.on_deliver_done(now.clone())?;
            let storage = self.storage.lock().unwrap();
            storage.update_session_state(
                &transition.session_id,
                transition.new_state.as_str(),
                &now,
            )?;
            Ok((transition, text_result))
        } else {
            Ok((
                StateTransition {
                    session_id,
                    prev_state: current_state.unwrap_or_default(),
                    new_state: SessionState::Idle,
                },
                text_result,
            ))
        }
    }

    /// rewrite_last: 最後のセグメントのテキストでリライトをトリガー
    /// 返り値: (segment_id, raw_text, mode)
    pub fn get_last_segment_for_rewrite(
        &self,
    ) -> Result<(String, String, Mode), AppError> {
        let (session_id, mode) = {
            let mgr = self.session_mgr.lock().unwrap();
            let s = mgr
                .active()
                .ok_or_else(|| AppError::internal("アクティブセッションがありません"))?;
            (s.session_id.clone(), s.mode)
        };

        let storage = self.storage.lock().unwrap();
        let detail = storage
            .get_session_detail(&session_id)?
            .ok_or_else(|| AppError::internal("セッション詳細が見つかりません"))?;

        let last_segment = detail
            .segments
            .last()
            .ok_or_else(|| AppError::internal("セグメントがありません"))?;

        Ok((
            last_segment.segment_id.clone(),
            last_segment.raw_text.clone(),
            mode,
        ))
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

    /// 辞書アップサート
    pub fn upsert_dictionary(&self, entry: DictionaryEntry) -> Result<String, AppError> {
        let storage = self.storage.lock().unwrap();
        storage.upsert_dictionary_entry(&entry)
    }

    /// 辞書一覧
    pub fn list_dictionary(&self, scope: Option<&str>) -> Result<Vec<DictionaryEntry>, AppError> {
        let storage = self.storage.lock().unwrap();
        storage.list_dictionary_entries(scope)
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
