use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;
use windows::Win32::Media::MediaFoundation::{
    IMFSensorActivitiesReport, IMFSensorActivitiesReportCallback,
    IMFSensorActivitiesReportCallback_Impl, IMFSensorActivityMonitor, IMFShutdown, MF_VERSION,
    MFCreateSensorActivityMonitor, MFSTARTUP_LITE, MFShutdown, MFStartup,
};
use windows::core::{Interface, Ref, Result};

use super::SharedState;

struct MediaFoundation;

impl MediaFoundation {
    fn new() -> Result<Self> {
        // SAFETY: The worker initializes COM first and balances this startup in Drop.
        unsafe {
            MFStartup(MF_VERSION, MFSTARTUP_LITE)?;
        }
        Ok(Self)
    }
}

impl Drop for MediaFoundation {
    fn drop(&mut self) {
        // SAFETY: The sensor monitor and callback have been released before this guard.
        unsafe {
            let _ = MFShutdown();
        }
    }
}

pub(super) struct CameraMonitor {
    monitor: IMFSensorActivityMonitor,
    _foundation: MediaFoundation,
}

impl CameraMonitor {
    pub(super) fn new(state: Arc<SharedState>) -> Result<Self> {
        let foundation = MediaFoundation::new()?;
        let callback: IMFSensorActivitiesReportCallback = CameraCallback {
            devices: Mutex::new(HashMap::new()),
            state: state.clone(),
        }
        .into();
        // SAFETY: The monitor retains the callback and both live in the worker's MTA.
        let monitor = unsafe { MFCreateSensorActivityMonitor(&callback)? };
        let monitor = Self {
            monitor,
            _foundation: foundation,
        };
        state.set(0, Some(false));
        // SAFETY: Media Foundation and COM are initialized until CameraMonitor is dropped.
        if let Err(error) = unsafe { monitor.monitor.Start() } {
            state.set(0, None);
            return Err(error);
        }
        Ok(monitor)
    }
}

impl Drop for CameraMonitor {
    fn drop(&mut self) {
        // SAFETY: Stop and Shutdown run before the monitor and Media Foundation are released.
        unsafe {
            let _ = self.monitor.Stop();
            if let Ok(shutdown) = self.monitor.cast::<IMFShutdown>() {
                let _ = shutdown.Shutdown();
            }
        }
    }
}

#[windows::core::implement(IMFSensorActivitiesReportCallback)]
struct CameraCallback {
    devices: Mutex<HashMap<String, bool>>,
    state: Arc<SharedState>,
}

impl IMFSensorActivitiesReportCallback_Impl for CameraCallback_Impl {
    fn OnActivitiesReport(&self, reports: Ref<'_, IMFSensorActivitiesReport>) -> Result<()> {
        let reports = reports.ok()?;
        let mut devices = self.devices.lock();
        // SAFETY: Report interfaces remain owned by the callback for this synchronous traversal.
        unsafe {
            for index in 0..reports.GetCount()? {
                let report = reports.GetActivityReport(index)?;
                let mut name = [0u16; 1024];
                let mut length = 0;
                report.GetSymbolicLink(&mut name, &mut length)?;
                let length = name.iter().position(|c| *c == 0).unwrap_or(name.len());
                let name = String::from_utf16_lossy(&name[..length]);
                let mut active = false;
                for process in 0..report.GetProcessCount()? {
                    if report
                        .GetProcessActivity(process)?
                        .GetStreamingState()?
                        .as_bool()
                    {
                        active = true;
                        break;
                    }
                }
                devices.insert(name, active);
            }
        }
        self.state
            .set(0, Some(devices.values().any(|active| *active)));
        Ok(())
    }
}
