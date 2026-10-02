pub(crate) mod activities;
mod geometry;

pub(crate) use geometry::{IslandSurface, surface};

use winisland_core::multitask::TaskFrame;
use winisland_render::Painter;

use activities::Activity;

pub(crate) fn side_extent(scale: f32, compact_height: f32) -> f32 {
    compact_height + 20.0 * scale
}

pub(crate) fn draw_content(
    painter: Painter<'_>,
    surface: &IslandSurface,
    frame: Option<TaskFrame<Activity>>,
    scale: f32,
) {
    if let (Some(rect), Some(frame)) = (surface.secondary, frame) {
        painter.save();
        painter.clip_path(&surface.outline);
        frame
            .task
            .content
            .draw(painter, rect, scale, frame.content_opacity);
        painter.restore();
    }
}
