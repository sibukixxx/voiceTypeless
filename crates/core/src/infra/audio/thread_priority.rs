/// 現在のスレッドを高優先度に設定する（オーディオコールバック用）。
///
/// macOS: pthread_setschedparam で SCHED_RR を設定。
/// 他のプラットフォーム: ベストエフォート。設定失敗時はログ出力のみ。
pub fn set_audio_thread_priority() {
    #[cfg(target_os = "macos")]
    {
        set_realtime_priority_macos();
    }
    #[cfg(not(target_os = "macos"))]
    {
        log::debug!("Audio thread priority elevation not implemented for this platform");
    }
}

#[cfg(target_os = "macos")]
fn set_realtime_priority_macos() {
    use std::mem::MaybeUninit;

    unsafe {
        let thread = libc::pthread_self();
        let mut policy = 0i32;
        let mut param = MaybeUninit::<libc::sched_param>::zeroed().assume_init();

        let ret = libc::pthread_getschedparam(thread, &mut policy, &mut param);
        if ret != 0 {
            log::warn!("Failed to get thread sched params: {}", ret);
            return;
        }

        // SCHED_RR (Round Robin) with elevated priority
        let max_priority = libc::sched_get_priority_max(libc::SCHED_RR);
        param.sched_priority = (max_priority / 2).max(1);

        let ret = libc::pthread_setschedparam(thread, libc::SCHED_RR, &param);
        if ret != 0 {
            // EPERM (1) is expected without root — not an error
            if ret == libc::EPERM {
                log::debug!("Audio thread priority: SCHED_RR requires elevated privileges, using default");
            } else {
                log::warn!("Failed to set audio thread priority: {}", ret);
            }
        } else {
            log::debug!("Audio thread priority set to SCHED_RR (priority={})", param.sched_priority);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_priority_does_not_panic() {
        // Should not panic regardless of permissions
        set_audio_thread_priority();
    }
}
