mod device;

pub(crate) use device::DeviceIndicators;

use winisland_render::{Painter, Rect};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Activity {
    Devices { camera: f32, microphone: f32 },
}

impl Activity {
    pub(crate) fn width(self, height: f32, _scale: f32) -> f32 {
        match self {
            Self::Devices { .. } => device::width(height),
        }
    }

    pub(crate) fn draw(self, painter: Painter<'_>, rect: Rect, scale: f32, opacity: f32) {
        match self {
            Self::Devices { camera, microphone } => {
                device::draw(painter, rect, scale, opacity, camera, microphone)
            }
        }
    }
}
