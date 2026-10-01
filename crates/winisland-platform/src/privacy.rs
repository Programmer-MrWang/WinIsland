use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrivacyDevice {
    Microphone,
    Camera,
    Location,
}

impl PrivacyDevice {
    pub const ALL: [Self; 3] = [Self::Microphone, Self::Camera, Self::Location];

    pub fn index(self) -> usize {
        match self {
            Self::Microphone => 0,
            Self::Camera => 1,
            Self::Location => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DeviceUsageState {
    #[default]
    Disabled,
    Unavailable,
    NotDetected,
    Active,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DeviceUsage {
    pub state: DeviceUsageState,
    pub apps: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PrivacySnapshot {
    pub devices: [DeviceUsage; 3],
}

impl PrivacySnapshot {
    pub fn has_activity(&self) -> bool {
        self.devices
            .iter()
            .any(|device| device.state == DeviceUsageState::Active)
    }
}

pub trait PrivacyMonitor {
    fn update(&mut self, devices: [bool; 3]) -> Option<PrivacySnapshot>;
}

pub trait PrivacyProvider {
    fn open_monitor(&self, wake: Arc<dyn Fn() + Send + Sync>) -> Box<dyn PrivacyMonitor>;
}
