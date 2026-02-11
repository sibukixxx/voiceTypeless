mod commands;
mod events;

use std::sync::Arc;

use vt_core::infra::storage::Storage;
use vt_core::infra::stt::SttEngine;
use vt_core::usecase::app_service::AppService;

/// STT エンジンを構築する（macOS: Apple Speech, 他: Noop）
fn create_stt_engine() -> Arc<dyn SttEngine> {
    #[cfg(target_os = "macos")]
    {
        use vt_core::infra::stt::apple_speech::AppleSttEngine;
        if AppleSttEngine::is_available() {
            log::info!("Apple Speech STT engine selected");
            return Arc::new(AppleSttEngine);
        }
        log::warn!("Apple Speech not available, falling back to Noop STT");
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
    let stt_engine = create_stt_engine();
    let app_service = AppService::new(storage, stt_engine);

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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
