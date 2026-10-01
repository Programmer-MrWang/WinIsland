use crate::platform::WindowRef;
use crate::ui::expanded::pager::ExpandedPage;

use super::App;

impl App {
    pub(super) fn update_privacy_monitor(&mut self, window: &WindowRef) {
        let devices = if self.config.device_usage_enabled {
            [
                self.config.device_usage_microphone,
                self.config.device_usage_camera,
                self.config.device_usage_location,
            ]
        } else {
            [false; 3]
        };
        if let Some(snapshot) = self.privacy_monitor.update(devices) {
            self.privacy_snapshot = snapshot;
            window.request_redraw();
        }
        if !self.config.device_usage_enabled && self.current_page == ExpandedPage::DeviceUsage {
            self.reset_page();
            self.snap_to_current_page();
        }
    }

    pub(super) fn privacy_indicator_width(&self) -> f32 {
        if !self.config.device_usage_enabled
            || self.components_hidden
            || self.compact_overlay.is_visible()
            || (!self.config.device_usage_during_music && self.current_media_info().is_playing)
        {
            return 0.0;
        }
        crate::ui::privacy::indicator_width(&self.privacy_snapshot) * self.config.compact_scale
    }

    pub(super) fn privacy_keeps_visible(&self) -> bool {
        self.config.device_usage_enabled
            && self.config.device_usage_keep_visible
            && !self.components_hidden
            && self.privacy_snapshot.has_activity()
    }
}
