mod camera;
mod microphone;

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, mpsc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use winisland_platform::{
    DeviceActivity, DeviceActivityFeed, DeviceActivityProvider, PlatformError,
};

use crate::com::ComGuard;

pub struct WindowsDeviceActivity;

struct SharedState {
    bits: AtomicU8,
    wake: fn(),
}

impl SharedState {
    fn set(&self, shift: u8, active: Option<bool>) {
        let value = match active {
            None => 0,
            Some(false) => 1,
            Some(true) => 2,
        };
        let mask = 3 << shift;
        let previous = self
            .bits
            .try_update(Ordering::AcqRel, Ordering::Acquire, |bits| {
                Some((bits & !mask) | (value << shift))
            })
            .unwrap_or_else(|bits| bits);
        if previous & mask != value << shift {
            (self.wake)();
        }
    }

    fn snapshot(&self) -> DeviceActivity {
        let bits = self.bits.load(Ordering::Acquire);
        let decode = |value| match value {
            1 => Some(false),
            2 => Some(true),
            _ => None,
        };
        DeviceActivity {
            camera: decode(bits & 3),
            microphone: decode((bits >> 2) & 3),
        }
    }
}

struct WindowsActivityFeed {
    state: Arc<SharedState>,
    stop: mpsc::Sender<()>,
    worker: Option<JoinHandle<()>>,
}

impl DeviceActivityProvider for WindowsDeviceActivity {
    fn monitor(&self, wake: fn()) -> Result<Box<dyn DeviceActivityFeed>, PlatformError> {
        let state = Arc::new(SharedState {
            bits: AtomicU8::new(0),
            wake,
        });
        let (stop, receiver) = mpsc::channel();
        let worker_state = state.clone();
        let worker = thread::Builder::new()
            .name("winisland-device-activity".into())
            .spawn(move || run(worker_state, receiver))
            .map_err(PlatformError::backend)?;
        Ok(Box::new(WindowsActivityFeed {
            state,
            stop,
            worker: Some(worker),
        }))
    }
}

impl DeviceActivityFeed for WindowsActivityFeed {
    fn snapshot(&self) -> DeviceActivity {
        self.state.snapshot()
    }
}

impl Drop for WindowsActivityFeed {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn run(state: Arc<SharedState>, stop: mpsc::Receiver<()>) {
    let _com = match ComGuard::mta() {
        Ok(com) => com,
        Err(error) => {
            log::warn!("Device activity monitor: {error}");
            return;
        }
    };
    let _camera = match camera::CameraMonitor::new(state.clone()) {
        Ok(camera) => Some(camera),
        Err(error) => {
            log::warn!("Camera activity monitor unavailable: {error}");
            None
        }
    };
    let mut microphone = match microphone::MicrophoneMonitor::new(state.clone()) {
        Ok(monitor) => Some(monitor),
        Err(error) => {
            log::warn!("Microphone activity monitor unavailable: {error}");
            None
        }
    };
    let mut refresh_at = Instant::now();
    loop {
        if let Some(monitor) = &mut microphone {
            let now = Instant::now();
            if now >= refresh_at {
                if let Err(error) = monitor.refresh() {
                    log::debug!("Microphone endpoint refresh: {error}");
                }
                refresh_at = now + Duration::from_secs(2);
            }
            state.set(2, monitor.active().ok());
        }
        match stop.recv_timeout(Duration::from_millis(100)) {
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            _ => break,
        }
    }
}
