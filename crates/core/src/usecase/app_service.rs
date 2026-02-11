use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use crate::domain::error::AppError;
use crate::domain::session::{SessionManager, SessionState, StateTransition};
use crate::domain::settings::AppSettings;
use crate::domain::types::{
    DeliverPolicy, DictionaryEntry, HistoryPage, Mode, SessionDetail,
};
use crate::infra::audio::pipeline::{AudioPipeline, PipelineEvent};
use crate::infra::audio::vad::VadConfig;
use crate::infra::metrics::{Metrics, MetricsSummary};
use crate::infra::os_integration::{OsIntegration, PasteResult, PasteRouter, PermissionStatus};
use crate::infra::output::OutputRouter;
use crate::infra::post_processor::PostProcessor;
use crate::infra::storage::Storage;
use crate::infra::stt::SttEngine;

/// アプリケーションサービス（Tauri State として管理される）
pub struct AppService {
    session_mgr: Mutex<SessionManager>,
    storage: Mutex<Storage>,
    output_router: OutputRouter,
    metrics: Metrics,
    stt_engine: Arc<dyn SttEngine>,
    pipeline: Mutex<Option<AudioPipeline>>,
}

impl AppService {
    pub fn new(storage: Storage, stt_engine: Arc<dyn SttEngine>) -> Self {
        Self {
            session_mgr: Mutex::new(SessionManager::new()),
            storage: Mutex::new(storage),
            output_router: OutputRouter::new(),
            metrics: Metrics::new(),
            stt_engine,
            pipeline: Mutex::new(None),
        }
    }

    // ==================== Session ====================

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

        self.metrics.inc_sessions_started();

        let transition = StateTransition {
            session_id: session_id.clone(),
            prev_state: "none".to_string(),
            new_state: SessionState::Idle,
        };

        Ok((session_id, transition))
    }

    pub fn stop_session(&self) -> Result<Option<StateTransition>, AppError> {
        // パイプラインが動作中なら停止
        self.stop_pipeline();

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

        if transition.new_state == SessionState::Transcribing {
            let segment_id = uuid::Uuid::new_v4().to_string();
            storage.insert_segment(&segment_id, &transition.session_id, &now)?;
        }

        Ok(transition)
    }

    /// パイプラインモード用: Recording → Idle
    pub fn pause_recording(&self) -> Result<StateTransition, AppError> {
        // パイプライン停止（最終セグメント処理完了まで待機）
        self.stop_pipeline();

        let now = chrono::Utc::now().to_rfc3339();
        let mut mgr = self.session_mgr.lock().unwrap();
        let transition = mgr.pause_recording(now.clone())?;

        let storage = self.storage.lock().unwrap();
        storage.update_session_state(
            &transition.session_id,
            transition.new_state.as_str(),
            &now,
        )?;

        Ok(transition)
    }

    pub fn set_mode(&self, mode: Mode) -> Result<(), AppError> {
        let mut mgr = self.session_mgr.lock().unwrap();
        mgr.set_mode(mode)
    }

    // ==================== Audio Pipeline ====================

    /// パイプラインを開始し、イベント受信チャネルを返す
    pub fn start_pipeline(
        &self,
    ) -> Result<mpsc::Receiver<PipelineEvent>, AppError> {
        let (event_tx, event_rx) = mpsc::channel();
        let pipeline = AudioPipeline::start(
            self.stt_engine.clone(),
            event_tx,
            VadConfig::default(),
        )
        .map_err(|e| AppError::device(e.to_string()))?;

        *self.pipeline.lock().unwrap() = Some(pipeline);
        Ok(event_rx)
    }

    /// パイプラインを停止する
    pub fn stop_pipeline(&self) {
        if let Some(mut pipeline) = self.pipeline.lock().unwrap().take() {
            pipeline.stop();
        }
    }

    /// パイプラインからの書き起こし結果を処理する
    /// セグメントをDBに保存し、ポストプロセス済みテキストを返す
    pub fn on_pipeline_transcript(
        &self,
        text: &str,
        confidence: f32,
    ) -> Result<String, AppError> {
        let session_id = self
            .current_session_id()
            .ok_or_else(|| AppError::internal("アクティブセッションがありません"))?;

        let segment_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        let storage = self.storage.lock().unwrap();
        storage.insert_segment(&segment_id, &session_id, &now)?;

        // ポストプロセス（正規化 + 辞書置換）
        let mode_str = {
            let mgr = self.session_mgr.lock().unwrap();
            mgr.active().and_then(|s| {
                serde_json::to_value(s.mode)
                    .ok()
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
            })
        };
        let dict_entries = storage
            .get_enabled_dictionary_entries("global", mode_str.as_deref())
            .unwrap_or_default();

        let processed_text = PostProcessor::process(text, &dict_entries);
        storage.update_segment_text(&segment_id, &processed_text, confidence)?;

        self.metrics.inc_segments_transcribed();

        Ok(processed_text)
    }

    // ==================== Pipeline (legacy) ====================

    pub fn on_transcript_done(
        &self,
        segment_id: &str,
        text: &str,
        confidence: f32,
    ) -> Result<(StateTransition, String), AppError> {
        let start = std::time::Instant::now();
        let now = chrono::Utc::now().to_rfc3339();

        let storage = self.storage.lock().unwrap();
        let mode_str = {
            let mgr = self.session_mgr.lock().unwrap();
            mgr.active()
                .and_then(|s| {
                    serde_json::to_value(s.mode)
                        .ok()
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                })
        };
        let dict_entries = storage
            .get_enabled_dictionary_entries("global", mode_str.as_deref())
            .unwrap_or_default();

        let processed_text = PostProcessor::process(text, &dict_entries);
        storage.update_segment_text(segment_id, &processed_text, confidence)?;

        let mut mgr = self.session_mgr.lock().unwrap();
        let transition = mgr.on_transcript_done(now.clone())?;
        storage.update_session_state(
            &transition.session_id,
            transition.new_state.as_str(),
            &now,
        )?;

        self.metrics.inc_segments_transcribed();
        self.metrics
            .record_latency("transcribe", start.elapsed().as_millis() as u64);

        Ok((transition, processed_text))
    }

    pub fn on_rewrite_done(
        &self,
        segment_id: &str,
        rewritten_text: &str,
    ) -> Result<StateTransition, AppError> {
        let start = std::time::Instant::now();
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

        self.metrics.inc_segments_rewritten();
        self.metrics
            .record_latency("rewrite", start.elapsed().as_millis() as u64);

        Ok(transition)
    }

    pub fn deliver(&self, text: &str) -> Result<StateTransition, AppError> {
        let start = std::time::Instant::now();
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

        self.metrics.inc_segments_delivered();
        self.metrics
            .record_latency("deliver", start.elapsed().as_millis() as u64);

        Ok(transition)
    }

    pub fn deliver_last(&self) -> Result<(StateTransition, String), AppError> {
        let start = std::time::Instant::now();

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

        let mgr = self.session_mgr.lock().unwrap();
        let current_state = mgr.active().map(|s| s.state.as_str().to_string());
        drop(mgr);

        let text_result = text.to_string();

        self.metrics.inc_segments_delivered();
        self.metrics
            .record_latency("deliver", start.elapsed().as_millis() as u64);

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

    // ==================== Paste Router ====================

    pub fn paste_to_active_app(&self, text: &str) -> Result<PasteResult, AppError> {
        let storage = self.storage.lock().unwrap();
        let settings = storage.get_settings()?;
        drop(storage);

        PasteRouter::paste_if_allowlisted(
            text,
            &settings.paste_allowlist,
            settings.paste_confirm,
        )
    }

    // ==================== Queries ====================

    pub fn get_history(
        &self,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<HistoryPage, AppError> {
        let storage = self.storage.lock().unwrap();
        storage.list_history(limit, cursor)
    }

    pub fn get_session(&self, session_id: &str) -> Result<Option<SessionDetail>, AppError> {
        let storage = self.storage.lock().unwrap();
        storage.get_session_detail(session_id)
    }

    // ==================== Dictionary ====================

    pub fn upsert_dictionary(&self, entry: DictionaryEntry) -> Result<String, AppError> {
        let storage = self.storage.lock().unwrap();
        storage.upsert_dictionary_entry(&entry)
    }

    pub fn list_dictionary(&self, scope: Option<&str>) -> Result<Vec<DictionaryEntry>, AppError> {
        let storage = self.storage.lock().unwrap();
        storage.list_dictionary_entries(scope)
    }

    // ==================== Settings ====================

    pub fn get_settings(&self) -> Result<AppSettings, AppError> {
        let storage = self.storage.lock().unwrap();
        storage.get_settings()
    }

    pub fn update_settings(&self, settings: AppSettings) -> Result<(), AppError> {
        let storage = self.storage.lock().unwrap();
        storage.save_settings(&settings)
    }

    // ==================== OS Integration ====================

    pub fn check_permissions(&self) -> PermissionStatus {
        OsIntegration::check_all_permissions()
    }

    // ==================== Metrics ====================

    pub fn get_metrics(&self) -> MetricsSummary {
        self.metrics.summary()
    }

    pub fn record_error(&self, code: &str) {
        self.metrics.inc_error(code);
    }

    // ==================== Data Protection ====================

    pub fn cleanup_old_data(&self, ttl_days: u32) -> Result<(u32, u32), AppError> {
        if ttl_days == 0 {
            return Ok((0, 0));
        }

        let cutoff = chrono::Utc::now()
            - chrono::Duration::days(ttl_days as i64);
        let cutoff_str = cutoff.to_rfc3339();

        let storage = self.storage.lock().unwrap();
        let segments_deleted = storage.delete_old_segments(&cutoff_str)?;
        let sessions_deleted = storage.delete_old_sessions(&cutoff_str)?;

        log::info!(
            "データクリーンアップ: {segments_deleted} セグメント、{sessions_deleted} セッション削除（TTL: {ttl_days}日）"
        );

        Ok((segments_deleted, sessions_deleted))
    }

    // ==================== State Accessors ====================

    pub fn current_state(&self) -> Option<String> {
        let mgr = self.session_mgr.lock().unwrap();
        mgr.active().map(|s| s.state.as_str().to_string())
    }

    pub fn current_session_id(&self) -> Option<String> {
        let mgr = self.session_mgr.lock().unwrap();
        mgr.active().map(|s| s.session_id.clone())
    }
}
