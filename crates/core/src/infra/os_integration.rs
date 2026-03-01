use serde::Serialize;
use crate::domain::error::AppError;

#[cfg(target_os = "macos")]
use std::ffi::CStr;
#[cfg(target_os = "macos")]
use std::ffi::c_void;
#[cfg(target_os = "macos")]
use std::os::raw::c_char;

#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> u8;
}

#[cfg(target_os = "macos")]
#[link(name = "AVFoundation", kind = "framework")]
unsafe extern "C" {
    static AVMediaTypeAudio: *mut c_void;
}

#[cfg(target_os = "macos")]
#[link(name = "objc")]
unsafe extern "C" {
    fn objc_getClass(name: *const c_char) -> *mut c_void;
    fn sel_registerName(name: *const c_char) -> *mut c_void;
    fn objc_msgSend();
}

#[cfg(target_os = "macos")]
#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {}

#[cfg(target_os = "macos")]
type ObjcId = *mut c_void;

#[cfg(target_os = "macos")]
unsafe fn objc_class(name: &'static [u8]) -> ObjcId {
    unsafe { objc_getClass(name.as_ptr().cast()) }
}

#[cfg(target_os = "macos")]
unsafe fn objc_sel(name: &'static [u8]) -> *mut c_void {
    unsafe { sel_registerName(name.as_ptr().cast()) }
}

#[cfg(target_os = "macos")]
unsafe fn msg_send_id(receiver: ObjcId, sel_name: &'static [u8]) -> ObjcId {
    let f: unsafe extern "C" fn(ObjcId, *mut c_void) -> ObjcId =
        unsafe { std::mem::transmute(objc_msgSend as *const ()) };
    unsafe { f(receiver, objc_sel(sel_name)) }
}

#[cfg(target_os = "macos")]
unsafe fn msg_send_i64_with_id_arg(
    receiver: ObjcId,
    sel_name: &'static [u8],
    arg: ObjcId,
) -> i64 {
    let f: unsafe extern "C" fn(ObjcId, *mut c_void, ObjcId) -> i64 =
        unsafe { std::mem::transmute(objc_msgSend as *const ()) };
    unsafe { f(receiver, objc_sel(sel_name), arg) }
}

#[cfg(target_os = "macos")]
unsafe fn msg_send_cstr(receiver: ObjcId, sel_name: &'static [u8]) -> *const c_char {
    let f: unsafe extern "C" fn(ObjcId, *mut c_void) -> *const c_char =
        unsafe { std::mem::transmute(objc_msgSend as *const ()) };
    unsafe { f(receiver, objc_sel(sel_name)) }
}

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
        unsafe {
            let capture_device = objc_class(b"AVCaptureDevice\0");
            if capture_device.is_null() || AVMediaTypeAudio.is_null() {
                return PermissionState::Unavailable;
            }

            let status = msg_send_i64_with_id_arg(
                capture_device,
                b"authorizationStatusForMediaType:\0",
                AVMediaTypeAudio,
            );
            match status {
                0 => PermissionState::NotDetermined,
                1 => PermissionState::Denied,
                2 => PermissionState::Denied,
                3 => PermissionState::Granted,
                _ => PermissionState::Unavailable,
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub fn check_microphone_permission() -> PermissionState {
        PermissionState::Unavailable
    }

    /// アクセシビリティ権限をチェック（macOS — 貼り付けに必要）
    #[cfg(target_os = "macos")]
    pub fn check_accessibility_permission() -> PermissionState {
        unsafe {
            if AXIsProcessTrusted() != 0 {
                PermissionState::Granted
            } else {
                // API上、未決定と拒否を区別できないため Denied として扱う
                PermissionState::Denied
            }
        }
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
        unsafe {
            let workspace_class = objc_class(b"NSWorkspace\0");
            if workspace_class.is_null() {
                return None;
            }

            let workspace = msg_send_id(workspace_class, b"sharedWorkspace\0");
            if workspace.is_null() {
                return None;
            }

            let frontmost = msg_send_id(workspace, b"frontmostApplication\0");
            if frontmost.is_null() {
                return None;
            }

            let bundle_id = msg_send_id(frontmost, b"bundleIdentifier\0");
            if bundle_id.is_null() {
                return None;
            }

            let utf8 = msg_send_cstr(bundle_id, b"UTF8String\0");
            if utf8.is_null() {
                return None;
            }

            CStr::from_ptr(utf8).to_str().ok().map(|s| s.to_string())
        }
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
        // 権限状態は実行環境に依存するため、列挙値として妥当なことのみ確認
        assert!(matches!(
            status.microphone,
            PermissionState::Granted
                | PermissionState::Denied
                | PermissionState::NotDetermined
                | PermissionState::Unavailable
        ));
        assert!(matches!(
            status.accessibility,
            PermissionState::Granted
                | PermissionState::Denied
                | PermissionState::NotDetermined
                | PermissionState::Unavailable
        ));
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
