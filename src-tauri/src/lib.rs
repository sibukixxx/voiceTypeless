mod commands;
mod events;

use std::sync::Arc;

use vt_core::domain::settings::SttEngineChoice;
use vt_core::infra::rewriter::Rewriter;
use vt_core::infra::storage::Storage;
use vt_core::infra::stt::SttEngine;
use vt_core::usecase::app_service::AppService;

/// リライターを構築する（API Key あり → Claude, なし → Noop）
fn create_rewriter(storage: &Storage) -> Arc<dyn Rewriter> {
    let api_key = storage
        .get_settings()
        .ok()
        .and_then(|s| s.claude_api_key)
        .unwrap_or_default();

    if !api_key.is_empty() {
        log::info!("Claude rewriter selected (API key configured)");
        Arc::new(vt_core::infra::rewriter::claude::ClaudeRewriter::new(api_key))
    } else {
        log::info!("Using Noop rewriter (no API key)");
        Arc::new(vt_core::infra::rewriter::NoopRewriter)
    }
}

/// STT エンジンを構築する（設定に応じて選択）
fn create_stt_engine(storage: &Storage) -> Arc<dyn SttEngine> {
    let settings = storage.get_settings().unwrap_or_default();

    match settings.stt_engine {
        SttEngineChoice::Apple => {
            #[cfg(target_os = "macos")]
            {
                use vt_core::infra::stt::apple_speech::AppleSttEngine;
                if AppleSttEngine::is_available() {
                    log::info!("Apple Speech STT engine selected");
                    return Arc::new(AppleSttEngine);
                }
                log::warn!("Apple Speech not available, falling back to Noop STT");
            }
            #[cfg(not(target_os = "macos"))]
            {
                log::warn!("Apple Speech is only available on macOS, falling back to Noop STT");
            }
        }
        SttEngineChoice::Whisper => {
            use vt_core::infra::stt::whisper::WhisperSttEngine;
            let model_path = WhisperSttEngine::default_model_path();
            if model_path.exists() {
                match WhisperSttEngine::new(&model_path.to_string_lossy()) {
                    Ok(engine) => {
                        log::info!("Whisper STT engine selected");
                        return Arc::new(engine);
                    }
                    Err(e) => {
                        log::error!("Whisper engine init failed: {}, falling back to Noop", e);
                    }
                }
            } else {
                log::warn!(
                    "Whisper model not found at {:?}, falling back to Noop STT",
                    model_path
                );
            }
        }
        SttEngineChoice::Cloud => {
            log::warn!("Cloud STT not yet implemented, falling back to Noop STT");
        }
    }

    log::info!("Using Noop STT engine");
    Arc::new(vt_core::infra::stt::NoopSttService)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // DB パスはアプリデータディレクトリに配置
    // 開発時は一時ファイルを使用
    let db_path = std::env::var("VT_DB_PATH").unwrap_or_else(|_| {
        let dir = dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("voiceTypeless");
        std::fs::create_dir_all(&dir).ok();
        dir.join("voicetypeless.db")
            .to_string_lossy()
            .to_string()
    });

    let storage = Storage::open(&db_path).expect("SQLite の初期化に失敗しました");
    let stt_engine = create_stt_engine(&storage);
    let rewriter = create_rewriter(&storage);
    let app_service = AppService::new(storage, stt_engine, rewriter);

    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::default().build())
        .manage(app_service)
        .invoke_handler(tauri::generate_handler![
            commands::start_session,
            commands::stop_session,
            commands::toggle_recording,
            commands::set_mode,
            commands::get_history,
            commands::get_session,
            commands::upsert_dictionary,
            commands::list_dictionary,
            commands::rewrite_last,
            commands::deliver_last,
            commands::get_settings,
            commands::update_settings,
            commands::check_permissions,
            commands::get_metrics,
            commands::cleanup_data,
            commands::paste_to_active_app,
            commands::check_whisper_model,
            commands::download_whisper_model,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
