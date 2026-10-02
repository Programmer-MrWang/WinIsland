use crate::PlatformError;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DeviceActivity {
    pub camera: Option<bool>,
    pub microphone: Option<bool>,
}

impl DeviceActivity {
    pub const fn any(self) -> bool {
        matches!(self.camera, Some(true)) || matches!(self.microphone, Some(true))
    }
}

pub trait DeviceActivityFeed {
    fn snapshot(&self) -> DeviceActivity;
}

pub trait DeviceActivityProvider {
    fn monitor(&self, wake: fn()) -> Result<Box<dyn DeviceActivityFeed>, PlatformError>;
}
