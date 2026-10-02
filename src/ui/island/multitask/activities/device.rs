use std::time::{Duration, Instant};

use winisland_core::anim::{Curve, Tween};
use winisland_platform::DeviceActivity;
use winisland_render::{Painter, Point, Rect, Rgba};

use super::Activity;

pub(crate) struct DeviceIndicators {
    camera: Tween,
    microphone: Tween,
}

impl Default for DeviceIndicators {
    fn default() -> Self {
        Self {
            camera: Tween::new(0.0),
            microphone: Tween::new(0.0),
        }
    }
}

impl DeviceIndicators {
    pub(crate) fn update(&mut self, state: DeviceActivity, now: Instant) -> bool {
        let before = self.activity();
        for (animation, active) in [
            (&mut self.camera, state.camera),
            (&mut self.microphone, state.microphone),
        ] {
            animation.retarget(
                if active == Some(true) { 1.0 } else { 0.0 },
                Duration::from_millis(140),
                Curve::Ease,
                now,
            );
            animation.update(now);
        }
        before != self.activity()
    }

    pub(crate) fn activity(&self) -> Option<Activity> {
        let camera = self.camera.value();
        let microphone = self.microphone.value();
        (camera > 0.0 || microphone > 0.0 || self.is_animating())
            .then_some(Activity::Devices { camera, microphone })
    }

    pub(crate) fn is_animating(&self) -> bool {
        self.camera.is_animating() || self.microphone.is_animating()
    }
}

pub(super) fn width(height: f32) -> f32 {
    height
}

pub(super) fn draw(
    painter: Painter<'_>,
    rect: Rect,
    scale: f32,
    opacity: f32,
    camera: f32,
    microphone: f32,
) {
    let dots = [
        (camera, -microphone, Rgba::from_rgb(48, 209, 88)),
        (microphone, camera, Rgba::from_rgb(255, 159, 10)),
    ];
    for (visibility, offset, color) in dots {
        let x = rect.center_x() + offset * 0.5 * (10.0 * scale).min(rect.width() * 0.35);
        painter.fill_circle(
            Point::new(x, rect.center_y()),
            3.0 * scale,
            color.with_alpha_f(opacity * visibility),
        );
    }
}
