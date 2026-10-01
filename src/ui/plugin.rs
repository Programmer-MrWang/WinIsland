use std::collections::HashMap;

use winisland_plugin_api::{SURFACE_COMPACT_LEFT, SURFACE_COMPACT_RIGHT};
use winisland_plugin_host::draw::replay::PreparedFrame;
use winisland_plugin_host::extensions::Presentation;
use winisland_plugin_host::host::PluginHost;
use winisland_render::{Painter, Rect};

pub struct SurfaceFrame {
    pub id: u64,
    pub rect: Rect,
    pub logical: [f32; 2],
    pub input_offset: f32,
    pub alpha: u8,
    pub interactive: bool,
}

pub fn draw(
    painter: Painter<'_>,
    host: &PluginHost,
    frames: &HashMap<u64, PreparedFrame>,
    surface: SurfaceFrame,
) {
    if surface.alpha == 0 {
        return;
    }
    let rect = surface.rect;
    host.present(Presentation {
        target: surface.id,
        bounds: [
            rect.left + surface.input_offset,
            rect.top,
            rect.width(),
            rect.height(),
        ],
        logical: surface.logical,
        interactive: surface.interactive,
    });
    if let Some(frame) = frames.get(&surface.id) {
        super::expanded::widget_view::draw_prepared_widget(
            painter,
            surface.id,
            frame,
            rect.left,
            rect.top,
            rect.width(),
            rect.height(),
            surface.alpha,
        );
    }
}

pub fn compact_widths(host: Option<&PluginHost>) -> (f32, f32) {
    let mut widths = (0.0, 0.0);
    if let Some(host) = host {
        for (_, spec) in host.surfaces() {
            let width = spec.width.clamp(24.0, 256.0) + 8.0;
            match spec.kind {
                SURFACE_COMPACT_LEFT => widths.0 += width,
                SURFACE_COMPACT_RIGHT => widths.1 += width,
                _ => {}
            }
        }
    }
    widths
}

pub fn draw_layer(
    painter: Painter<'_>,
    host: &PluginHost,
    frames: &HashMap<u64, PreparedFrame>,
    kind: u32,
    rect: Rect,
    scale: f32,
    alpha: u8,
) {
    for (id, spec) in host
        .surfaces()
        .into_iter()
        .filter(|(_, spec)| spec.kind == kind)
    {
        let logical = [rect.width() / scale, rect.height() / scale];
        let _ = spec;
        draw(
            painter,
            host,
            frames,
            SurfaceFrame {
                id,
                rect,
                logical,
                input_offset: 0.0,
                alpha,
                interactive: alpha > 240,
            },
        );
    }
}

pub fn draw_compact(
    painter: Painter<'_>,
    host: &PluginHost,
    frames: &HashMap<u64, PreparedFrame>,
    rect: Rect,
    scale: f32,
    alpha: u8,
) {
    let mut left = rect.left + 4.0 * scale;
    let mut right = rect.right - 4.0 * scale;
    for (id, spec) in host.surfaces() {
        let width = spec.width.clamp(24.0, 256.0);
        let x = match spec.kind {
            SURFACE_COMPACT_LEFT => {
                let x = left;
                left += (width + 8.0) * scale;
                x
            }
            SURFACE_COMPACT_RIGHT => {
                right -= width * scale;
                let x = right;
                right -= 8.0 * scale;
                x
            }
            _ => continue,
        };
        draw(
            painter,
            host,
            frames,
            SurfaceFrame {
                id,
                rect: Rect::from_xywh(x, rect.top, width * scale, rect.height()),
                logical: [width, rect.height() / scale],
                input_offset: 0.0,
                alpha,
                interactive: alpha > 240,
            },
        );
    }
}
