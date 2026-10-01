use std::collections::HashSet;
use std::path::Path;
use std::sync::{Arc, mpsc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use parking_lot::Mutex;
use windows::Win32::Media::Audio::{
    AudioSessionStateActive, DEVICE_STATE_ACTIVE, IAudioSessionControl2, IAudioSessionManager2,
    IMMDeviceEnumerator, MMDeviceEnumerator, eCapture,
};
use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance};
use windows::Win32::System::SystemInformation::GetTickCount64;
use windows::core::Interface;
use windows_registry::{CURRENT_USER, Key};
use winisland_platform::{
    DeviceUsage, DeviceUsageState, PrivacyDevice, PrivacyMonitor, PrivacyProvider, PrivacySnapshot,
};

use crate::com::ComGuard;
use crate::process;

const CONSENT_STORE: &str =
    r"Software\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore";
const POLL_INTERVAL: Duration = Duration::from_secs(1);
const FILETIME_UNIX_OFFSET: u64 = 116_444_736_000_000_000;

#[derive(Default)]
struct MonitorState {
    revision: u64,
    devices: [bool; 3],
    snapshot: PrivacySnapshot,
}

pub struct WindowsPrivacy;

impl PrivacyProvider for WindowsPrivacy {
    fn open_monitor(&self, wake: Arc<dyn Fn() + Send + Sync>) -> Box<dyn PrivacyMonitor> {
        Box::new(WindowsPrivacyMonitor {
            wake,
            devices: [false; 3],
            shared: Arc::new(Mutex::new(MonitorState::default())),
            revision: 0,
            commands: None,
            worker: None,
        })
    }
}

struct WindowsPrivacyMonitor {
    wake: Arc<dyn Fn() + Send + Sync>,
    devices: [bool; 3],
    shared: Arc<Mutex<MonitorState>>,
    revision: u64,
    commands: Option<mpsc::Sender<[bool; 3]>>,
    worker: Option<JoinHandle<()>>,
}

impl WindowsPrivacyMonitor {
    fn stop(&mut self) {
        self.commands.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }

    fn start(&mut self) {
        let (sender, receiver) = mpsc::channel();
        let shared = Arc::clone(&self.shared);
        let wake = Arc::clone(&self.wake);
        let devices = self.devices;
        match thread::Builder::new()
            .name("device-usage".into())
            .spawn(move || {
                monitor_devices(receiver, shared, wake, devices);
            }) {
            Ok(worker) => {
                self.commands = Some(sender);
                self.worker = Some(worker);
            }
            Err(error) => log::warn!("Cannot start device usage monitor: {error}"),
        }
    }
}

impl PrivacyMonitor for WindowsPrivacyMonitor {
    fn update(&mut self, devices: [bool; 3]) -> Option<PrivacySnapshot> {
        if devices != self.devices {
            self.devices = devices;
            let snapshot = PrivacySnapshot {
                devices: devices.map(|enabled| DeviceUsage {
                    state: if enabled {
                        DeviceUsageState::Unavailable
                    } else {
                        DeviceUsageState::Disabled
                    },
                    apps: Vec::new(),
                }),
            };
            let mut shared = self.shared.lock();
            shared.revision = shared.revision.wrapping_add(1);
            shared.devices = devices;
            shared.snapshot = snapshot;
            drop(shared);
            if let Some(sender) = &self.commands {
                let _ = sender.send(devices);
            } else if devices.iter().any(|enabled| *enabled) {
                self.start();
            }
        }
        let shared = self.shared.lock();
        if shared.revision == self.revision {
            None
        } else {
            self.revision = shared.revision;
            let mut snapshot = shared.snapshot.clone();
            for (index, enabled) in self.devices.iter().enumerate() {
                if !enabled {
                    snapshot.devices[index] = DeviceUsage::default();
                }
            }
            Some(snapshot)
        }
    }
}

impl Drop for WindowsPrivacyMonitor {
    fn drop(&mut self) {
        self.stop();
    }
}

fn monitor_devices(
    receiver: mpsc::Receiver<[bool; 3]>,
    shared: Arc<Mutex<MonitorState>>,
    wake: Arc<dyn Fn() + Send + Sync>,
    mut devices: [bool; 3],
) {
    let com = ComGuard::mta().ok();
    loop {
        while let Ok(options) = receiver.try_recv() {
            devices = options;
        }
        if !devices.iter().any(|enabled| *enabled) {
            match receiver.recv() {
                Ok(options) => {
                    devices = options;
                    continue;
                }
                Err(_) => break,
            }
        }
        let mut snapshot = PrivacySnapshot::default();
        let mut running = None;
        for device in PrivacyDevice::ALL {
            if devices[device.index()] {
                snapshot.devices[device.index()] = registry_usage(device, &mut running);
            }
        }
        if devices[0] && com.is_some() {
            let capture = microphone_usage();
            let microphone = &mut snapshot.devices[0];
            if capture.state == DeviceUsageState::Active {
                microphone.apps.extend(capture.apps);
                microphone.state = DeviceUsageState::Active;
            } else if microphone.state != DeviceUsageState::Active {
                *microphone = capture;
            }
            normalize_apps(&mut microphone.apps);
        }
        let mut latest = shared.lock();
        let changed = latest.devices == devices && latest.snapshot != snapshot;
        if changed {
            latest.revision = latest.revision.wrapping_add(1);
            latest.snapshot = snapshot;
        }
        drop(latest);
        if changed {
            wake();
        }
        match receiver.recv_timeout(POLL_INTERVAL) {
            Ok(options) => devices = options,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
}

fn normalize_apps(apps: &mut Vec<String>) {
    apps.sort_unstable_by_key(|name| name.to_lowercase());
    apps.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
}

fn registry_usage(device: PrivacyDevice, running: &mut Option<HashSet<String>>) -> DeviceUsage {
    let capability = match device {
        PrivacyDevice::Microphone => "microphone",
        PrivacyDevice::Camera => "webcam",
        PrivacyDevice::Location => "location",
    };
    let Ok(key) = CURRENT_USER.open(format!(r"{CONSENT_STORE}\{capability}")) else {
        return DeviceUsage {
            state: DeviceUsageState::Unavailable,
            apps: Vec::new(),
        };
    };
    let mut usage = DeviceUsage {
        state: DeviceUsageState::NotDetected,
        apps: Vec::new(),
    };
    let mut failed = false;
    read_usage_records(&key, 0, boot_filetime(), &mut usage.apps, &mut failed);
    usage.apps.retain(|name| {
        let running = running.get_or_insert_with(process::running_app_identities);
        let identity = name.replace('#', "\\").to_lowercase();
        if running.contains(&identity) {
            true
        } else {
            failed = true;
            false
        }
    });
    for name in &mut usage.apps {
        *name = registry_app_name(name);
    }
    normalize_apps(&mut usage.apps);
    usage.state = if !usage.apps.is_empty() {
        DeviceUsageState::Active
    } else if failed {
        DeviceUsageState::Unavailable
    } else {
        DeviceUsageState::NotDetected
    };
    usage
}

fn boot_filetime() -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        / 100;
    // SAFETY: GetTickCount64 reads the system uptime and has no pointer arguments.
    let uptime = unsafe { GetTickCount64() };
    FILETIME_UNIX_OFFSET
        .saturating_add(now as u64)
        .saturating_sub(uptime.saturating_mul(10_000))
}

fn read_usage_records(key: &Key, depth: u8, boot: u64, apps: &mut Vec<String>, failed: &mut bool) {
    let Ok(children) = key.keys() else {
        *failed = true;
        return;
    };
    for name in children {
        let Ok(child) = key.open(&name) else {
            *failed = true;
            continue;
        };
        let start = child.get_u64("LastUsedTimeStart");
        let stop = child.get_u64("LastUsedTimeStop");
        match (start, stop) {
            (Ok(start), Ok(0)) if start > 0 && start >= boot => {
                apps.push(name);
            }
            (Ok(start), Ok(0)) if start > 0 => *failed = true,
            (Ok(_), Ok(_)) => {}
            _ if depth < 2 => read_usage_records(&child, depth + 1, boot, apps, failed),
            _ => *failed = true,
        }
    }
}

fn registry_app_name(name: &str) -> String {
    if name.contains('#') {
        let decoded = name.replace('#', "\\");
        Path::new(&decoded)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or(decoded)
    } else {
        name.split('_').next().unwrap_or(name).to_string()
    }
}

fn microphone_usage() -> DeviceUsage {
    let mut usage = DeviceUsage {
        state: DeviceUsageState::Unavailable,
        apps: Vec::new(),
    };
    // SAFETY: The endpoint and session interfaces are created and used only on this COM-initialized worker.
    unsafe {
        let Ok(enumerator) =
            CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL)
        else {
            return usage;
        };
        let Ok(devices) = enumerator.EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE) else {
            return usage;
        };
        let Ok(count) = devices.GetCount() else {
            return usage;
        };
        let mut failed = false;
        let mut active = false;
        for index in 0..count {
            let sessions = devices
                .Item(index)
                .and_then(|device| device.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None))
                .and_then(|manager| manager.GetSessionEnumerator());
            let Ok(sessions) = sessions else {
                failed = true;
                continue;
            };
            let Ok(count) = sessions.GetCount() else {
                failed = true;
                continue;
            };
            for session in 0..count {
                let Ok(control) = sessions.GetSession(session) else {
                    failed = true;
                    continue;
                };
                match control.GetState() {
                    Ok(state) if state == AudioSessionStateActive => {
                        active = true;
                        if let Ok(control) = control.cast::<IAudioSessionControl2>()
                            && let Ok(pid) = control.GetProcessId()
                            && let Some(handle) = process::open(pid)
                            && let Some(name) = process::executable_name(*handle)
                        {
                            usage.apps.push(name);
                        }
                    }
                    Ok(_) => {}
                    Err(_) => failed = true,
                }
            }
        }
        usage.state = if active {
            DeviceUsageState::Active
        } else if failed {
            DeviceUsageState::Unavailable
        } else {
            DeviceUsageState::NotDetected
        };
    }
    usage
}
