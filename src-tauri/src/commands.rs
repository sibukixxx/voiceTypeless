use std::sync::mpsc;

use serde::Deserialize;
use tauri::{AppHandle, State};

use vt_core::domain::session::SessionState;
use vt_core::domain::settings::AppSettings;
use vt_core::domain::types::{
    DeliverPolicy, DictionaryEntry, HistoryPage, Mode, SessionDetail,
};
use vt_core::infra::audio::pipeline::PipelineEvent;
use vt_core::infra::metrics::MetricsSummary;
use vt_core::infra::os_integration::{PasteResult, PermissionStatus};
use vt_core::usecase::app_service::AppService;

use crate::events::{
    self, AudioLevelPayload, ErrorPayload, SessionStateChangedPayload,
    TranscriptFinalPayload, TranscriptPartialPayload,
    AUDIO_LEVEL, DELIVER_DONE, ERROR, REWRITE_DONE, SESSION_STATE_CHANGED,
    TRANSCRIPT_FINAL, TRANSCRIPT_PARTIAL,
};

/// コマンドエラー型（Tauri の Result で使用）
#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("{0}")]
    App(#[from] vt_core::domain::error::AppError),
}

impl serde::Serialize for CommandError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

type CmdResult<T> = Result<T, CommandError>;

// --- Commands ---

#[tauri::command(rename_all = "camelCase")]
pub fn start_session(
    app: AppHandle,
    service: State<'_, AppService>,
    mode: Mode,
    deliver_policy: DeliverPolicy,
) -> CmdResult<String> {
    log::info!("start_session called: mode={:?}", mode);
    let (session_id, transition) = service.start_session(mode, deliver_policy)?;

    events::emit_event(
        &app,
        SESSION_STATE_CHANGED,
        SessionStateChangedPayload {
            session_id: session_id.clone(),
            prev_state: transition.prev_state,
            new_state: transition.new_state.as_str().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        },
    );

    Ok(session_id)
}

#[tauri::command]
pub fn stop_session(
    app: AppHandle,
    service: State<'_, AppService>,
) -> CmdResult<()> {
    let transition = service.stop_session()?;

    if let Some(t) = transition {
        events::emit_event(
            &app,
            SESSION_STATE_CHANGED,
            SessionStateChangedPayload {
                session_id: t.session_id,
                prev_state: t.prev_state,
                new_state: t.new_state.as_str().to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            },
        );
    }

    Ok(())
}

#[tauri::command]
pub fn toggle_recording(
    app: AppHandle,
    service: State<'_, AppService>,
) -> CmdResult<()> {
    let current_state = service.current_state();

    if current_state.as_deref() == Some("recording") {
        // 録音中 → 一時停止（パイプライン停止 + Recording→Idle）
        let transition = service.pause_recording()?;
        events::emit_event(
            &app,
            SESSION_STATE_CHANGED,
            SessionStateChangedPayload {
                session_id: transition.session_id,
                prev_state: transition.prev_state,
                new_state: transition.new_state.as_str().to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            },
        );
    } else {
        // Idle → Recording（パイプライン開始）
        let transition = service.toggle_recording()?;
        events::emit_event(
            &app,
            SESSION_STATE_CHANGED,
            SessionStateChangedPayload {
                session_id: transition.session_id.clone(),
                prev_state: transition.prev_state.clone(),
                new_state: transition.new_state.as_str().to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            },
        );

        if transition.new_state == SessionState::Recording {
            match service.start_pipeline() {
                Ok(event_rx) => {
                    spawn_event_forwarder(app.clone(), event_rx);
                }
                Err(e) => {
                    log::error!("Failed to start audio pipeline: {}", e);
                    // パイプライン開始失敗 → Idle に戻す
                    if let Ok(revert) = service.pause_recording() {
                        events::emit_event(
                            &app,
                            SESSION_STATE_CHANGED,
                            SessionStateChangedPayload {
                                session_id: revert.session_id.clone(),
                                prev_state: revert.prev_state,
                                new_state: revert.new_state.as_str().to_string(),
                                timestamp: chrono::Utc::now().to_rfc3339(),
                            },
                        );
                    }
                    events::emit_event(
                        &app,
                        ERROR,
                        ErrorPayload {
                            code: "E_DEVICE".to_string(),
                            message: e.to_string(),
                            recoverable: true,
                            session_id: service.current_session_id(),
                        },
                    );
                    return Err(e.into());
                }
            }
        }
    }

    Ok(())
}

/// パイプラインイベント → Tauri イベントの転送スレッドを起動
fn spawn_event_forwarder(app: AppHandle, event_rx: mpsc::Receiver<PipelineEvent>) {
    std::thread::spawn(move || {
        use tauri::Manager;
        for event in event_rx {
            match event {
                PipelineEvent::AudioLevel(rms) => {
                    events::emit_event(&app, AUDIO_LEVEL, AudioLevelPayload { rms });
                }
                PipelineEvent::TranscriptPartial { text } => {
                    events::emit_event(
                        &app,
                        TRANSCRIPT_PARTIAL,
                        TranscriptPartialPayload { text },
                    );
                }
                PipelineEvent::TranscriptFinal { text, confidence } => {
                    let service = app.state::<AppService>();
                    match service.on_pipeline_transcript(&text, confidence) {
                        Ok(processed_text) => {
                            events::emit_event(
                                &app,
                                TRANSCRIPT_FINAL,
                                TranscriptFinalPayload {
                                    text: processed_text,
                                    confidence,
                                },
                            );
                        }
                        Err(e) => {
                            log::error!("Pipeline transcript processing error: {}", e);
                        }
                    }
                }
                PipelineEvent::Error(msg) => {
                    events::emit_event(
                        &app,
                        ERROR,
                        ErrorPayload {
                            code: "E_PIPELINE".to_string(),
                            message: msg,
                            recoverable: true,
                            session_id: None,
                        },
                    );
                }
            }
        }
        log::info!("Pipeline event forwarder thread exiting");
    });
}

#[tauri::command]
pub fn set_mode(
    service: State<'_, AppService>,
    mode: Mode,
) -> CmdResult<()> {
    service.set_mode(mode)?;
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetHistoryArgs {
    #[serde(default)]
    query: Option<String>,
    limit: u32,
    #[serde(default)]
    cursor: Option<String>,
}

#[tauri::command]
pub fn get_history(
    service: State<'_, AppService>,
    args: GetHistoryArgs,
) -> CmdResult<HistoryPage> {
    let _ = args.query;
    let page = service.get_history(args.limit, args.cursor.as_deref())?;
    Ok(page)
}

#[tauri::command]
pub fn get_session(
    service: State<'_, AppService>,
    session_id: String,
) -> CmdResult<Option<SessionDetail>> {
    let detail = service.get_session(&session_id)?;
    Ok(detail)
}

#[tauri::command]
pub fn upsert_dictionary(
    service: State<'_, AppService>,
    entry: DictionaryEntry,
) -> CmdResult<String> {
    let id = service.upsert_dictionary(entry)?;
    Ok(id)
}

#[tauri::command]
pub fn list_dictionary(
    service: State<'_, AppService>,
    scope: Option<String>,
) -> CmdResult<Vec<DictionaryEntry>> {
    let entries = service.list_dictionary(scope.as_deref())?;
    Ok(entries)
}

#[tauri::command]
pub fn rewrite_last(
    app: AppHandle,
    service: State<'_, AppService>,
    mode: Mode,
) -> CmdResult<()> {
    let (segment_id, raw_text, _current_mode) = service.get_last_segment_for_rewrite()?;

    // NoopRewriter相当: Phase3でLLM連携に差し替え
    let rewritten = format!("[rewritten:{}] {}", serde_json::to_value(mode).unwrap_or_default(), raw_text);

    service.on_rewrite_done(&segment_id, &rewritten)?;

    let session_id = service.current_session_id().unwrap_or_default();
    events::emit_event(
        &app,
        REWRITE_DONE,
        events::RewriteDonePayload {
            session_id,
            segment_id,
            text: rewritten,
            mode: serde_json::to_value(mode)
                .ok()
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_default(),
        },
    );

    Ok(())
}

#[tauri::command]
pub fn deliver_last(
    app: AppHandle,
    service: State<'_, AppService>,
) -> CmdResult<()> {
    let (transition, _text) = service.deliver_last()?;

    events::emit_event(
        &app,
        DELIVER_DONE,
        events::DeliverDonePayload {
            session_id: transition.session_id.clone(),
            target: "clipboard".to_string(),
        },
    );

    events::emit_event(
        &app,
        SESSION_STATE_CHANGED,
        SessionStateChangedPayload {
            session_id: transition.session_id,
            prev_state: transition.prev_state,
            new_state: transition.new_state.as_str().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        },
    );

    Ok(())
}

// --- Phase 3 Commands ---

#[tauri::command]
pub fn get_settings(
    service: State<'_, AppService>,
) -> CmdResult<AppSettings> {
    let settings = service.get_settings()?;
    Ok(settings)
}

#[tauri::command]
pub fn update_settings(
    service: State<'_, AppService>,
    settings: AppSettings,
) -> CmdResult<()> {
    service.update_settings(settings)?;
    Ok(())
}

#[tauri::command]
pub fn check_permissions(
    service: State<'_, AppService>,
) -> CmdResult<PermissionStatus> {
    Ok(service.check_permissions())
}

#[tauri::command]
pub fn get_metrics(
    service: State<'_, AppService>,
) -> CmdResult<MetricsSummary> {
    Ok(service.get_metrics())
}

#[tauri::command]
pub fn cleanup_data(
    service: State<'_, AppService>,
    ttl_days: u32,
) -> CmdResult<(u32, u32)> {
    let result = service.cleanup_old_data(ttl_days)?;
    Ok(result)
}

#[tauri::command]
pub fn paste_to_active_app(
    service: State<'_, AppService>,
    text: String,
) -> CmdResult<PasteResult> {
    let result = service.paste_to_active_app(&text)?;
    Ok(result)
}
