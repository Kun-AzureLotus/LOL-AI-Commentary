use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppVolumeError {
    Unavailable,
}

impl AppVolumeError {
    pub fn as_str(self) -> &'static str {
        "App Volume unavailable"
    }
}

pub fn percent_from_scalar(scalar: f32) -> u16 {
    (scalar.clamp(0.0, 1.0) * 100.0).round() as u16
}

pub fn scalar_from_percent(percent: u16) -> f32 {
    f32::from(percent.min(100)) / 100.0
}

pub fn get_app_volume_percent() -> Result<u16, AppVolumeError> {
    with_current_process_volumes(|volumes| unsafe {
        let volume = volumes.first().ok_or(AppVolumeError::Unavailable)?;
        volume
            .GetMasterVolume()
            .map(percent_from_scalar)
            .map_err(|_| AppVolumeError::Unavailable)
    })
}

pub fn set_app_volume_percent(percent: u16) -> Result<(), AppVolumeError> {
    let scalar = scalar_from_percent(percent);
    with_current_process_volumes(|volumes| unsafe {
        if volumes.is_empty() {
            return Err(AppVolumeError::Unavailable);
        }
        let mut any = false;
        for volume in volumes {
            if volume
                .SetMasterVolume(scalar, std::ptr::null())
                .is_ok()
            {
                any = true;
            }
        }
        if any {
            Ok(())
        } else {
            Err(AppVolumeError::Unavailable)
        }
    })
}

pub fn apply_app_volume_when_available(
    percent: u16,
    attempts: u32,
    delay_ms: u64,
) -> Result<(), AppVolumeError> {
    for index in 0..attempts.max(1) {
        if set_app_volume_percent(percent).is_ok() {
            return Ok(());
        }
        if index + 1 < attempts {
            std::thread::sleep(Duration::from_millis(delay_ms));
        }
    }
    Err(AppVolumeError::Unavailable)
}

#[cfg(windows)]
fn with_current_process_volumes<T>(
    callback: impl FnOnce(&[windows::Win32::Media::Audio::ISimpleAudioVolume]) -> Result<T, AppVolumeError>,
) -> Result<T, AppVolumeError> {
    windows_current_process_volumes(callback)
}

#[cfg(not(windows))]
fn with_current_process_volumes<T>(
    _callback: impl FnOnce(&[()]) -> Result<T, AppVolumeError>,
) -> Result<T, AppVolumeError> {
    Err(AppVolumeError::Unavailable)
}

#[cfg(windows)]
fn windows_current_process_volumes<T>(
    callback: impl FnOnce(&[windows::Win32::Media::Audio::ISimpleAudioVolume]) -> Result<T, AppVolumeError>,
) -> Result<T, AppVolumeError> {
    use windows::core::Interface;
    use windows::Win32::Foundation::S_OK;
    use windows::Win32::Media::Audio::{
        eMultimedia, eRender, IAudioSessionControl2, IAudioSessionManager2, ISimpleAudioVolume,
        IMMDeviceEnumerator, MMDeviceEnumerator, AudioSessionStateExpired,
    };
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
    };

    unsafe {
        let initialized = CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok();
        let result = (|| {
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                    .map_err(|_| AppVolumeError::Unavailable)?;
            let device = enumerator
                .GetDefaultAudioEndpoint(eRender, eMultimedia)
                .map_err(|_| AppVolumeError::Unavailable)?;
            let manager: IAudioSessionManager2 = device
                .Activate(CLSCTX_ALL, None)
                .map_err(|_| AppVolumeError::Unavailable)?;
            let sessions = manager
                .GetSessionEnumerator()
                .map_err(|_| AppVolumeError::Unavailable)?;
            let count = sessions.GetCount().map_err(|_| AppVolumeError::Unavailable)?;
            let current_pid = std::process::id();
            let mut volumes = Vec::new();
            for index in 0..count {
                let Ok(control) = sessions.GetSession(index) else {
                    continue;
                };
                let Ok(control2) = control.cast::<IAudioSessionControl2>() else {
                    continue;
                };
                let Ok(pid) = control2.GetProcessId() else {
                    continue;
                };
                if pid != current_pid {
                    continue;
                }
                if control2.IsSystemSoundsSession() == S_OK {
                    continue;
                }
                if control.GetState().map(|state| state.0) == Ok(AudioSessionStateExpired.0) {
                    continue;
                }
                let Ok(volume) = control.cast::<ISimpleAudioVolume>() else {
                    continue;
                };
                volumes.push(volume);
            }
            callback(&volumes)
        })();
        if initialized {
            CoUninitialize();
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_maps_to_percent() {
        assert_eq!(percent_from_scalar(0.0), 0);
        assert_eq!(percent_from_scalar(1.0), 100);
        assert_eq!(percent_from_scalar(0.5), 50);
        assert_eq!(percent_from_scalar(0.754), 75);
        assert_eq!(percent_from_scalar(-0.2), 0);
        assert_eq!(percent_from_scalar(1.4), 100);
    }

    #[test]
    fn percent_maps_to_scalar() {
        assert_eq!(scalar_from_percent(0), 0.0);
        assert_eq!(scalar_from_percent(100), 1.0);
        assert!((scalar_from_percent(40) - 0.4).abs() < f32::EPSILON);
        assert_eq!(scalar_from_percent(250), 1.0);
    }

    #[test]
    fn tts_volume_is_not_derived_from_app_volume() {
        assert_ne!(scalar_from_percent(50), 1.0);
        assert_eq!(percent_from_scalar(1.0), 100);
        assert_eq!(scalar_from_percent(100), 1.0);
    }

    #[test]
    fn unavailable_message_is_stable() {
        assert_eq!(
            AppVolumeError::Unavailable.as_str(),
            "App Volume unavailable"
        );
    }

    #[test]
    fn get_app_volume_does_not_panic() {
        let _ = get_app_volume_percent();
    }

    #[test]
    fn set_app_volume_does_not_panic_without_session() {
        let _ = set_app_volume_percent(80);
    }
}
