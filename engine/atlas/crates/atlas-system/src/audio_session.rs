//! Windows mixer access for the audio session of the selected media source.
use std::{
    os::windows::io::{FromRawHandle, OwnedHandle},
    path::Path,
    ptr,
};
use windows::{
    Win32::{
        Foundation::{ERROR_SUCCESS, S_FALSE, S_OK},
        Media::Audio::{
            AudioSessionStateActive, AudioSessionStateInactive, DEVICE_STATE_ACTIVE,
            IAudioSessionControl2, IAudioSessionManager2, IMMDeviceEnumerator, ISimpleAudioVolume,
            MMDeviceEnumerator, eRender,
        },
        Storage::Packaging::Appx::GetApplicationUserModelId,
        System::{
            Com::{
                CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree,
                CoUninitialize,
            },
            Threading::{
                OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
                QueryFullProcessImageNameW,
            },
        },
    },
    core::{Interface, PWSTR},
};

/// The Windows mixer. Keep it and its targets on the owning audio worker.
pub struct Mixer(Option<IMMDeviceEnumerator>);

pub struct Target {
    pub id: String,
    control: ISimpleAudioVolume,
}

impl Target {
    pub fn level(&self) -> Option<f32> {
        let value = unsafe { self.control.GetMasterVolume().ok()? };
        (0.0..=1.0).contains(&value).then_some(value)
    }

    pub fn set_level(&self, value: f32) {
        if (0.0..=1.0).contains(&value) {
            let _ = unsafe { self.control.SetMasterVolume(value, ptr::null()) };
        }
    }
}

impl Mixer {
    pub fn new() -> Option<Self> {
        unsafe { CoInitializeEx(None, COINIT_MULTITHREADED).ok().ok()? };
        let enumerator = unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok() };
        Some(Self(enumerator))
    }

    pub fn target(&self, source: &str) -> Option<Target> {
        if source.is_empty() {
            return None;
        }
        let enumerator = self.0.as_ref()?;
        let devices = unsafe {
            enumerator
                .EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)
                .ok()?
        };
        let mut active = None;
        let mut inactive = None;
        let mut multiple_inactive = false;
        for i in 0..unsafe { devices.GetCount().ok()? } {
            let Some(sessions) = (|| {
                let device = unsafe { devices.Item(i).ok()? };
                let manager: IAudioSessionManager2 =
                    unsafe { device.Activate(CLSCTX_ALL, None).ok()? };
                unsafe { manager.GetSessionEnumerator().ok() }
            })() else {
                continue;
            };
            for j in 0..unsafe { sessions.GetCount().unwrap_or(0) } {
                let Some((target, playing)) = (|| {
                    let session = unsafe { sessions.GetSession(j).ok()? };
                    let state = unsafe { session.GetState().ok()? };
                    if state != AudioSessionStateActive && state != AudioSessionStateInactive {
                        return None;
                    }
                    let identity = session.cast::<IAudioSessionControl2>().ok()?;
                    if unsafe { identity.IsSystemSoundsSession() } != S_FALSE
                        || !process_matches(&identity, source)
                    {
                        return None;
                    }
                    let instance = unsafe { identity.GetSessionInstanceIdentifier().ok()? };
                    if instance.is_null() {
                        return None;
                    }
                    let id = unsafe { instance.to_string() };
                    unsafe { CoTaskMemFree(Some(instance.0.cast())) };
                    let id = id.ok().filter(|id| !id.is_empty())?;
                    let control = session.cast::<ISimpleAudioVolume>().ok()?;
                    Some((Target { id, control }, state == AudioSessionStateActive))
                })() else {
                    continue;
                };
                if playing {
                    if active.replace(target).is_some() {
                        return None;
                    }
                } else {
                    multiple_inactive |= inactive.replace(target).is_some();
                }
            }
        }
        active.or(inactive.filter(|_| !multiple_inactive))
    }
}

impl Drop for Mixer {
    fn drop(&mut self) {
        drop(self.0.take());
        unsafe { CoUninitialize() };
    }
}

fn process_matches(session: &IAudioSessionControl2, source: &str) -> bool {
    let mut pid = 0;
    // The wrapper would also accept AUDCLNT_S_NO_SINGLE_PROCESS.
    let result = unsafe { (session.vtable().GetProcessId)(session.as_raw(), &mut pid) };
    if result != S_OK || pid == 0 {
        return false;
    }
    let Ok(process) = (unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) })
    else {
        return false;
    };
    let _owned = unsafe { OwnedHandle::from_raw_handle(process.0) };
    let mut app = [0u16; 256];
    let mut app_len = app.len() as u32;
    if unsafe { GetApplicationUserModelId(process, &mut app_len, Some(PWSTR(app.as_mut_ptr()))) }
        == ERROR_SUCCESS
        && let Some(app) = app.get(..app_len.saturating_sub(1) as usize)
        && String::from_utf16_lossy(app).eq_ignore_ascii_case(source)
    {
        return true;
    }
    let mut image = [0u16; 1024];
    let mut len = image.len() as u32;
    unsafe {
        QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(image.as_mut_ptr()),
            &mut len,
        )
    }
    .is_ok()
        && matches_executable(source, &String::from_utf16_lossy(&image[..len as usize]))
}

fn matches_executable(source: &str, image: &str) -> bool {
    if source.contains(['\\', '/']) {
        return source.eq_ignore_ascii_case(image);
    }
    if source.contains('!') {
        return false;
    }
    let path = Path::new(image);
    let name = path.file_name().and_then(|file| file.to_str());
    let stem = path.file_stem().and_then(|file| file.to_str());
    name.is_some_and(|name| name.eq_ignore_ascii_case(source))
        || stem.is_some_and(|stem| stem.eq_ignore_ascii_case(source))
        || (source.eq_ignore_ascii_case("Microsoft.MicrosoftEdge")
            && name.is_some_and(|name| name.eq_ignore_ascii_case("msedge.exe")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classic_executables_can_match_but_partial_and_wrong_package_ids_never_do() {
        let chrome = r"C:\Program Files\Google\Chrome\Application\chrome.exe";
        assert!(matches_executable("Chrome", chrome));
        assert!(matches_executable("chrome.exe", chrome));
        assert!(matches_executable(chrome, chrome));
        assert!(!matches_executable("chr", chrome));
        assert!(!matches_executable("Chrome!App", chrome));
        assert!(!matches_executable(r"C:\Other\chrome.exe", chrome));
        assert!(!matches_executable("Chrome", r"C:\Other\chrome-helper.exe"));
    }
}
