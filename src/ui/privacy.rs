use winisland_core::i18n::tr;
use winisland_platform::{DeviceUsageState, PrivacyDevice, PrivacySnapshot};
use winisland_render::text::{DrawTextCachedParams, FontManager};
use winisland_render::{FontStyle, Painter, PathBuilder, Point, Radius, Rect, Rgba};

pub(crate) const MAX_INDICATOR_WIDTH: f32 = 46.0;

fn color(device: PrivacyDevice) -> Rgba {
    match device {
        PrivacyDevice::Microphone => Rgba::from_rgb(255, 159, 10),
        PrivacyDevice::Camera => Rgba::from_rgb(48, 209, 88),
        PrivacyDevice::Location => Rgba::from_rgb(10, 132, 255),
    }
}

fn indicator_count(snapshot: &PrivacySnapshot) -> usize {
    snapshot
        .devices
        .iter()
        .filter(|device| device.state == DeviceUsageState::Active)
        .count()
        + usize::from(
            snapshot
                .devices
                .iter()
                .any(|device| device.state == DeviceUsageState::Unavailable),
        )
}

pub(crate) fn indicator_width(snapshot: &PrivacySnapshot) -> f32 {
    let count = indicator_count(snapshot);
    if count == 0 {
        0.0
    } else {
        count as f32 * 12.0 + 10.0
    }
}

pub(crate) fn indicator_rect(snapshot: &PrivacySnapshot, island: Rect, scale: f32) -> Rect {
    let width = indicator_width(snapshot) * scale;
    Rect::from_xywh(island.right - width, island.top, width, island.height())
}

fn draw_indicator(
    painter: Painter<'_>,
    device: PrivacyDevice,
    center: Point,
    scale: f32,
    alpha: f32,
) {
    let radius = if device == PrivacyDevice::Location {
        5.0
    } else {
        2.7
    } * scale;
    painter.fill_circle(center, radius, color(device).with_alpha_f(alpha));
    if device == PrivacyDevice::Location {
        let mut path = PathBuilder::new();
        for (index, (x, y)) in [(3.0, -3.0), (0.4, 3.0), (-0.7, 0.7), (-3.0, -0.4)]
            .into_iter()
            .enumerate()
        {
            let point = Point::new(center.x + x * scale, center.y + y * scale);
            if index == 0 {
                path.move_to(point);
            } else {
                path.line_to(point);
            }
        }
        path.close();
        painter.fill_path(&path.detach(), Rgba::WHITE.with_alpha_f(alpha));
    }
}

pub(crate) fn draw_indicators(
    painter: Painter<'_>,
    snapshot: &PrivacySnapshot,
    island: Rect,
    scale: f32,
    alpha: f32,
) {
    if alpha <= 0.01 {
        return;
    }
    let rect = indicator_rect(snapshot, island, scale);
    let mut x = rect.left + 6.0 * scale;
    for device in PrivacyDevice::ALL {
        if snapshot.devices[device.index()].state == DeviceUsageState::Active {
            draw_indicator(
                painter,
                device,
                Point::new(x, rect.center_y()),
                scale,
                alpha,
            );
            x += 12.0 * scale;
        }
    }
    if snapshot
        .devices
        .iter()
        .any(|device| device.state == DeviceUsageState::Unavailable)
    {
        draw_text(
            painter,
            "?",
            Rect::from_xywh(
                x - 3.0 * scale,
                rect.center_y() - 5.5 * scale,
                9.0 * scale,
                12.0 * scale,
            ),
            10.0 * scale,
            Rgba::from_rgb(174, 174, 178).with_alpha_f(alpha),
            false,
        );
    }
}

fn page_scale(rect: Rect, scale: f32) -> f32 {
    scale
        .min(rect.width() / 280.0)
        .min(rect.height() / 190.0)
        .max(0.01)
}

pub(crate) fn settings_rect(rect: Rect, scale: f32) -> Rect {
    let scale = page_scale(rect, scale);
    Rect::from_xywh(
        rect.right - 96.0 * scale,
        rect.bottom - 38.0 * scale,
        72.0 * scale,
        24.0 * scale,
    )
}

pub(crate) fn draw_page(
    painter: Painter<'_>,
    snapshot: &PrivacySnapshot,
    rect: Rect,
    scale: f32,
    alpha: u8,
) {
    let scale = page_scale(rect, scale);
    let inner = rect.inset(24.0 * scale);
    let foreground = Rgba::WHITE.with_alpha(alpha);
    let secondary = Rgba::from_rgb(174, 174, 178).with_alpha(alpha);
    draw_text(
        painter,
        &tr("device_usage_title"),
        Rect::from_xywh(inner.left, inner.top, inner.width(), 24.0 * scale),
        16.0 * scale,
        foreground,
        true,
    );
    let row_height = ((inner.height() - 46.0 * scale) / 3.0).max(28.0 * scale);
    for device in PrivacyDevice::ALL {
        let usage = &snapshot.devices[device.index()];
        let y = inner.top + 28.0 * scale + device.index() as f32 * row_height;
        let row = Rect::from_xywh(inner.left, y, inner.width(), row_height);
        let state_key = match usage.state {
            DeviceUsageState::Active => "device_usage_active",
            DeviceUsageState::NotDetected => "device_usage_not_detected",
            DeviceUsageState::Unavailable => "device_usage_unavailable",
            DeviceUsageState::Disabled => "device_usage_disabled",
        };
        let label_key = match device {
            PrivacyDevice::Microphone => "device_usage_microphone",
            PrivacyDevice::Camera => "device_usage_camera",
            PrivacyDevice::Location => "device_usage_location",
        };
        let opacity = f32::from(alpha) / 255.0;
        if usage.state == DeviceUsageState::Active {
            draw_indicator(
                painter,
                device,
                Point::new(row.left + 6.0 * scale, y + 10.0 * scale),
                scale,
                opacity,
            );
        } else {
            painter.fill_circle(
                Point::new(row.left + 6.0 * scale, y + 10.0 * scale),
                2.7 * scale,
                secondary,
            );
        }
        let state = tr(state_key);
        let state_width = FontManager::global()
            .measure_text_cached(&state, 10.0 * scale, FontStyle::normal())
            .min(inner.width() * 0.5);
        let text_left = row.left + 20.0 * scale;
        draw_text(
            painter,
            &tr(label_key),
            Rect::from_xywh(
                text_left,
                y + 2.0 * scale,
                row.right - state_width - text_left - 8.0 * scale,
                16.0 * scale,
            ),
            12.0 * scale,
            foreground,
            false,
        );
        draw_text(
            painter,
            &state,
            Rect::from_xywh(
                row.right - state_width,
                y + 3.0 * scale,
                state_width,
                15.0 * scale,
            ),
            10.0 * scale,
            secondary,
            false,
        );
        if usage.state == DeviceUsageState::Active {
            let apps = if usage.apps.is_empty() {
                tr("device_usage_unknown_app")
            } else {
                usage.apps.join(", ")
            };
            draw_text(
                painter,
                &apps,
                Rect::from_xywh(
                    text_left,
                    y + 19.0 * scale,
                    row.right - text_left,
                    13.0 * scale,
                ),
                10.0 * scale,
                secondary,
                false,
            );
        }
    }
    let settings = settings_rect(rect, scale);
    painter.fill_round_rect(
        settings,
        Radius::uniform(6.0 * scale),
        Rgba::from_rgb(48, 48, 52).with_alpha(alpha),
    );
    draw_text(
        painter,
        &tr("device_usage_settings"),
        Rect::from_xywh(
            settings.left + 8.0 * scale,
            settings.top + 5.0 * scale,
            settings.width() - 16.0 * scale,
            settings.height() - 8.0 * scale,
        ),
        10.0 * scale,
        foreground,
        false,
    );
    draw_text(
        painter,
        &tr("device_usage_details_hint"),
        Rect::from_xywh(
            inner.left,
            settings.top + 6.0 * scale,
            (settings.left - inner.left - 8.0 * scale).max(0.0),
            14.0 * scale,
        ),
        9.0 * scale,
        secondary,
        false,
    );
}

fn draw_text(painter: Painter<'_>, text: &str, rect: Rect, size: f32, color: Rgba, bold: bool) {
    if rect.width() <= 0.0 {
        return;
    }
    let manager = FontManager::global();
    let style = if bold {
        FontStyle::bold()
    } else {
        FontStyle::normal()
    };
    let mut text = text.to_string();
    if manager.measure_text_cached(&text, size, style) > rect.width() {
        let ellipsis = manager.measure_text_cached("…", size, style);
        let mut ends: Vec<_> = text.char_indices().map(|(index, _)| index).collect();
        ends.push(text.len());
        let mut low = 0;
        let mut high = ends.len() - 1;
        while low < high {
            let middle = (low + high).div_ceil(2);
            let (width, _) =
                manager.measure_text_prefixes_cached(&text, ends[middle], 0, size, style);
            if width + ellipsis <= rect.width() {
                low = middle;
            } else {
                high = middle - 1;
            }
        }
        text.truncate(ends[low]);
        text.push('…');
    }
    painter.save();
    painter.clip_rect(rect);
    manager.draw_text_cached(DrawTextCachedParams {
        painter,
        text: &text,
        x: rect.left,
        y: rect.top + size,
        size,
        bold,
        color,
        blur: None,
    });
    painter.restore();
}
