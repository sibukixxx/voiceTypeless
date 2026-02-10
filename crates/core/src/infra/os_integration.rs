use serde::Serialize;
use crate::domain::error::AppError;

/// OS権限の状態
#[derive(Debug, Clone, Serialize)]
pub struct PermissionStatus {
    pub microphone: PermissionState,
    pub accessibility: PermissionState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionState {
    Granted,
    Denied,
    NotDetermined,
    Unavailable,
}

/// OS権限チェッカー
pub struct OsIntegration;

impl OsIntegration {
    /// マイク権限をチェック（macOS）
    #[cfg(target_os = "macos")]
    pub fn check_microphone_permission() -> PermissionState {
        // macOS ではAVCaptureDevice.authorizationStatusをチェックする必要があるが、
        // Rustから直接呼ぶにはobjcブリッジが必要。
        // Phase3では権限チェック自体のインターフェースを提供し、
        // 実際のネイティブ呼び出しはTauri pluginまたはSwiftブリッジで行う。
        // ここではスタブとして NotDetermined を返す。
        log::info!("マイク権限チェック（macOS stub）");
        PermissionState::NotDetermined
    }

    #[cfg(not(target_os = "macos"))]
    pub fn check_microphone_permission() -> PermissionState {
        PermissionState::Unavailable
    }

    /// アクセシビリティ権限をチェック（macOS — 貼り付けに必要）
    #[cfg(target_os = "macos")]
    pub fn check_accessibility_permission() -> PermissionState {
        // AXIsProcessTrusted() を呼ぶ
        // Rustから直接呼べるが、安全のためスタブ
        log::info!("アクセシビリティ権限チェック（macOS stub）");
        PermissionState::NotDetermined
    }

    #[cfg(not(target_os = "macos"))]
    pub fn check_accessibility_permission() -> PermissionState {
        PermissionState::Unavailable
    }

    /// 全権限を一括チェック
    pub fn check_all_permissions() -> PermissionStatus {
        PermissionStatus {
            microphone: Self::check_microphone_permission(),
            accessibility: Self::check_accessibility_permission(),
        }
    }

    /// アクティブアプリのBundle IDを取得（macOS）
    #[cfg(target_os = "macos")]
    pub fn get_active_app_bundle_id() -> Option<String> {
        // NSWorkspace.shared.frontmostApplication?.bundleIdentifier
        // objcブリッジが必要。スタブとして None を返す。
        log::debug!("アクティブアプリ取得（macOS stub）");
        None
    }

    #[cfg(not(target_os = "macos"))]
    pub fn get_active_app_bundle_id() -> Option<String> {
        None
    }
}

/// ペーストルーター: allowlist ベースのペースト制御
pub struct PasteRouter;

impl PasteRouter {
    /// allowlist にアクティブアプリが含まれているかチェックし、ペーストを実行
    pub fn paste_if_allowlisted(
        text: &str,
        allowlist: &[String],
        require_confirm: bool,
    ) -> Result<PasteResult, AppError> {
        let bundle_id = OsIntegration::get_active_app_bundle_id();

        let Some(ref bid) = bundle_id else {
            // Bundle IDが取得できない場合はクリップボードのみ
            return Ok(PasteResult::FallbackClipboard {
                reason: "アクティブアプリを識別できません".to_string(),
            });
        };

        if allowlist.is_empty() || !allowlist.iter().any(|a| a == bid) {
            return Ok(PasteResult::FallbackClipboard {
                reason: format!("{bid} はallowlistに含まれていません"),
            });
        }

        if require_confirm {
            return Ok(PasteResult::NeedsConfirmation {
                app_bundle_id: bid.clone(),
                text: text.to_string(),
            });
        }

        // 実際のペースト（Cmd+V シミュレーション）はOS統合が必要
        // Phase3ではインターフェースだけ。実装はSwiftブリッジで行う。
        log::info!("ペースト実行: {bid} ({} 文字)", text.len());
        Ok(PasteResult::Pasted {
            app_bundle_id: bid.clone(),
        })
    }
}

/// ペースト結果
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PasteResult {
    Pasted { app_bundle_id: String },
    NeedsConfirmation { app_bundle_id: String, text: String },
    FallbackClipboard { reason: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_all_permissions() {
        let status = OsIntegration::check_all_permissions();
        // CI/テスト環境では Unavailable or NotDetermined
        assert!(
            status.microphone == PermissionState::NotDetermined
                || status.microphone == PermissionState::Unavailable
        );
    }

    #[test]
    fn test_paste_empty_allowlist() {
        let result = PasteRouter::paste_if_allowlisted("test", &[], false).unwrap();
        match result {
            PasteResult::FallbackClipboard { .. } => {}
            _ => panic!("空のallowlistではFallbackClipboardになるべき"),
        }
    }

    #[test]
    fn test_paste_result_serialization() {
        let result = PasteResult::FallbackClipboard {
            reason: "test".to_string(),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("fallback_clipboard"));
        assert!(json.contains("test"));
    }
}
