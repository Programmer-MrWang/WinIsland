use std::time::{Duration, Instant};

use winisland_core::multitask::{Presentation, Task, TaskId};
use winisland_platform::DeviceActivity;

use crate::platform::WindowRef;

use super::App;

impl App {
    pub(super) fn update_multitask(&mut self, window: &WindowRef, now: Instant) {
        if !self.config.device_status_indicators {
            self.device_feed = None;
            self.device_monitor_retry.clear();
        } else if self.device_feed.is_none() && self.device_monitor_retry.is_ready(now) {
            match crate::platform::device_activity().monitor(crate::platform::wake) {
                Ok(feed) => self.device_feed = Some(feed),
                Err(error) => {
                    log::warn!("Cannot start device activity monitor: {error}");
                    self.device_monitor_retry
                        .start(now, Duration::from_secs(10));
                }
            }
        }
        let activity = self
            .device_feed
            .as_ref()
            .map_or(DeviceActivity::default(), |feed| feed.snapshot());
        let indicators_changed = self.device_indicators.update(activity, now);
        let task = self.device_indicators.activity().map(|content| Task {
            id: TaskId(1),
            content,
        });
        self.multitask.set_secondary(task);
        let presentation = if self.expanded {
            Presentation::Expanded
        } else if !self.visible
            || self.is_hidden()
            || self.components_hidden
            || self.compact_overlay.is_visible()
        {
            Presentation::Hidden
        } else {
            Presentation::Compact
        };
        let compact_settled = !self.springs.expanded_target
            && (self.springs.h.value - self.compact_content_height()).abs()
                < 0.5 * self.config.compact_scale
            && self.springs.w.velocity.abs() < 0.1
            && self.springs.h.velocity.abs() < 0.1
            && self.springs.r.velocity.abs() < 0.1;
        let separation_before = self.multitask.frame().map_or(0.0, |frame| frame.separation);
        let changed = self.multitask.update(now, presentation, compact_settled);
        let separation = self.multitask.frame().map_or(0.0, |frame| frame.separation);
        let impulse = if separation_before >= 0.7 && separation < 0.7 {
            -1.3
        } else {
            0.0
        };
        if impulse != 0.0 {
            let direction = if self.compute_island_layout().secondary_left {
                -1.0
            } else {
                1.0
            };
            self.springs.multitask_x.velocity += impulse * direction * self.config.compact_scale;
        }
        if changed || indicators_changed {
            window.request_redraw();
        }
    }
}
