use std::collections::HashSet;
use std::sync::{Arc, mpsc};

use windows::Win32::Media::Audio::{
    AudioSessionStateActive, AudioSessionStateExpired, DEVICE_STATE_ACTIVE, IAudioSessionControl,
    IAudioSessionManager2, IAudioSessionNotification, IAudioSessionNotification_Impl,
    IMMDeviceEnumerator, MMDeviceEnumerator, eCapture,
};
use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance, CoTaskMemFree};
use windows::core::{AgileReference, Ref, Result};

use super::SharedState;

pub(super) struct MicrophoneMonitor {
    enumerator: IMMDeviceEnumerator,
    endpoints: Vec<Endpoint>,
    state: Arc<SharedState>,
}

impl MicrophoneMonitor {
    pub(super) fn new(state: Arc<SharedState>) -> Result<Self> {
        // SAFETY: This constructor is only called from the COM-initialized worker.
        let enumerator = unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)? };
        Ok(Self {
            enumerator,
            endpoints: Vec::new(),
            state,
        })
    }

    pub(super) fn refresh(&mut self) -> Result<()> {
        let mut ids = HashSet::new();
        // SAFETY: The endpoint collection and interfaces are used on their creating MTA worker.
        unsafe {
            let devices = self
                .enumerator
                .EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE)?;
            for index in 0..devices.GetCount()? {
                let device = devices.Item(index)?;
                let name = device.GetId()?;
                let id = name.to_string();
                CoTaskMemFree(Some(name.0.cast()));
                let id = id?;
                ids.insert(id.clone());
                if self.endpoints.iter().any(|endpoint| endpoint.id == id) {
                    continue;
                }
                let manager: IAudioSessionManager2 = device.Activate(CLSCTX_ALL, None)?;
                let (sender, receiver) = mpsc::channel();
                let callback: IAudioSessionNotification = SessionCallback {
                    sender,
                    state: self.state.clone(),
                }
                .into();
                manager.RegisterSessionNotification(&callback)?;
                let mut endpoint = Endpoint {
                    id,
                    manager,
                    callback,
                    receiver,
                    sessions: Vec::new(),
                };
                let sessions = endpoint.manager.GetSessionEnumerator()?;
                for i in 0..sessions.GetCount()? {
                    endpoint.sessions.push(sessions.GetSession(i)?);
                }
                self.endpoints.push(endpoint);
            }
        }
        self.endpoints.retain(|endpoint| ids.contains(&endpoint.id));
        Ok(())
    }

    pub(super) fn active(&mut self) -> Result<bool> {
        let mut active = false;
        for endpoint in &mut self.endpoints {
            for session in endpoint.receiver.try_iter() {
                endpoint.sessions.push(session.resolve()?);
            }
            // SAFETY: Sessions are resolved into and retained on this worker's apartment.
            unsafe {
                endpoint.sessions.retain(|session| {
                    session
                        .GetState()
                        .is_ok_and(|state| state != AudioSessionStateExpired)
                });
                for session in &endpoint.sessions {
                    active |= session.GetState()? == AudioSessionStateActive;
                }
            }
        }
        Ok(active)
    }
}

struct Endpoint {
    id: String,
    manager: IAudioSessionManager2,
    callback: IAudioSessionNotification,
    receiver: mpsc::Receiver<AgileReference<IAudioSessionControl>>,
    sessions: Vec<IAudioSessionControl>,
}

impl Drop for Endpoint {
    fn drop(&mut self) {
        // SAFETY: The callback is unregistered while both retained interfaces are still live.
        unsafe {
            let _ = self.manager.UnregisterSessionNotification(&self.callback);
        }
    }
}

#[windows::core::implement(IAudioSessionNotification)]
struct SessionCallback {
    sender: mpsc::Sender<AgileReference<IAudioSessionControl>>,
    state: Arc<SharedState>,
}

impl IAudioSessionNotification_Impl for SessionCallback_Impl {
    fn OnSessionCreated(&self, session: Ref<'_, IAudioSessionControl>) -> Result<()> {
        let reference = AgileReference::new(session.ok()?)?;
        let _ = self.sender.send(reference);
        (self.state.wake)();
        Ok(())
    }
}
