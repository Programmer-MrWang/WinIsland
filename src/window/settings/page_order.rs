use std::time::Instant;

use winisland_core::config::ExpandedPageKind;
use winisland_core::i18n::tr;
use winisland_render::text::FontManager;
use winisland_render::{Painter, Point, Radius, Rect, Rgba, StrokeCap};

use super::{SETTINGS_HEADER_H, SIDEBAR_W, SettingsApp, WIDGETS_PAGE_INDEX, animate_towards};
use crate::utils::color::SettingsTheme;
use crate::utils::settings_ui::items::{
    CONTENT_PADDING, GROUP_INNER_PAD, GROUP_RADIUS, ROW_HEIGHT, SettingsItem,
};
use crate::utils::settings_ui::{WidgetEditorMode, settings_color};

const SLIDE_RATE: f32 = 16.0;
const LIFT_RATE: f32 = 18.0;
const ARROW_BUTTON: f32 = 28.0;
const ARROW_GAP: f32 = 6.0;
const PRESS_SECS: f32 = 0.25;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ArrowDirection {
    Up,
    Down,
}

#[derive(Clone, Copy)]
pub(crate) struct PageOrderDrag {
    kind: ExpandedPageKind,
    grab_offset: f32,
}

pub(crate) struct PageOrderState {
    slots: [f32; ExpandedPageKind::ALL.len()],
    initialized: bool,
    drag: Option<PageOrderDrag>,
    lift: f32,
    press: Option<(ExpandedPageKind, ArrowDirection, Instant)>,
}

impl Default for PageOrderState {
    fn default() -> Self {
        Self {
            slots: [0.0; ExpandedPageKind::ALL.len()],
            initialized: false,
            drag: None,
            lift: 0.0,
            press: None,
        }
    }
}

fn kind_index(kind: ExpandedPageKind) -> usize {
    ExpandedPageKind::ALL
        .iter()
        .position(|candidate| *candidate == kind)
        .unwrap_or(0)
}

pub(crate) fn page_name_key(kind: ExpandedPageKind) -> &'static str {
    match kind {
        ExpandedPageKind::Music => "page_music",
        ExpandedPageKind::Widgets => "page_widgets",
        ExpandedPageKind::Calendar => "page_calendar",
        ExpandedPageKind::Timer => "page_timer",
    }
}

pub(crate) fn page_order_list_height(count: usize) -> f32 {
    count as f32 * ROW_HEIGHT
}

impl SettingsApp {
    fn page_order_active(&self) -> bool {
        self.active_page == WIDGETS_PAGE_INDEX
            && self.widget_editor_mode == WidgetEditorMode::Pages
            && !self.resource_editor_open
    }

    fn page_order_list_rect(&self) -> Option<Rect> {
        if !self.page_order_active() {
            return None;
        }
        let mut y = SETTINGS_HEADER_H;
        for item in &self.cached_items {
            if let SettingsItem::Custom { height } = item {
                return Some(Rect::from_xywh(
                    SIDEBAR_W + CONTENT_PADDING,
                    y - self.scroll_y,
                    self.content_width() - CONTENT_PADDING * 2.0,
                    *height,
                ));
            }
            y += item.height();
        }
        None
    }

    fn page_order_arrow_rect(list: Rect, row: usize, direction: ArrowDirection) -> Rect {
        let top = list.top + row as f32 * ROW_HEIGHT + (ROW_HEIGHT - ARROW_BUTTON) / 2.0;
        let right = list.right - GROUP_INNER_PAD;
        let left = match direction {
            ArrowDirection::Down => right - ARROW_BUTTON,
            ArrowDirection::Up => right - ARROW_BUTTON * 2.0 - ARROW_GAP,
        };
        Rect::from_xywh(left, top, ARROW_BUTTON, ARROW_BUTTON)
    }

    fn page_order_row_at(&mut self, x: f32, y: f32) -> Option<(Rect, usize)> {
        if !self.page_order_active() {
            return None;
        }
        self.ensure_items_cache();
        let list = self.page_order_list_rect()?;
        let point = Point::new(x, y);
        if !list.contains(point) || y < SETTINGS_HEADER_H {
            return None;
        }
        let row = ((y - list.top) / ROW_HEIGHT).floor() as usize;
        (row < self.config.expanded_page_order.len()).then_some((list, row))
    }

    pub(crate) fn page_order_hovered(&mut self, x: f32, y: f32) -> bool {
        self.page_order_row_at(x, y).is_some()
    }

    fn move_page(&mut self, from: usize, to: usize) {
        let order = &mut self.config.expanded_page_order;
        if from == to || from >= order.len() || to >= order.len() {
            return;
        }
        let kind = order.remove(from);
        order.insert(to, kind);
    }

    pub(crate) fn handle_page_order_press(&mut self) -> bool {
        let (x, y) = self.logical_mouse_pos;
        let Some((list, row)) = self.page_order_row_at(x, y) else {
            return false;
        };
        let count = self.config.expanded_page_order.len();
        let kind = self.config.expanded_page_order[row];
        for direction in [ArrowDirection::Up, ArrowDirection::Down] {
            if !Self::page_order_arrow_rect(list, row, direction).contains(Point::new(x, y)) {
                continue;
            }
            let target = match direction {
                ArrowDirection::Up => row.checked_sub(1),
                ArrowDirection::Down => (row + 1 < count).then_some(row + 1),
            };
            if let Some(target) = target {
                self.page_order.press = Some((kind, direction, Instant::now()));
                self.move_page(row, target);
                self.persist_settings_change();
            }
            return true;
        }
        let row_top = list.top + row as f32 * ROW_HEIGHT;
        self.page_order.drag = Some(PageOrderDrag {
            kind,
            grab_offset: y - row_top,
        });
        self.request_redraw();
        true
    }

    pub(crate) fn update_page_order_drag(&mut self) -> bool {
        let Some(drag) = self.page_order.drag else {
            return false;
        };
        self.ensure_items_cache();
        let Some(list) = self.page_order_list_rect() else {
            return false;
        };
        let count = self.config.expanded_page_order.len();
        let pointer_slot = (self.logical_mouse_pos.1 - drag.grab_offset - list.top) / ROW_HEIGHT;
        let target = pointer_slot
            .round()
            .clamp(0.0, count.saturating_sub(1) as f32) as usize;
        if let Some(current) = self
            .config
            .expanded_page_order
            .iter()
            .position(|kind| *kind == drag.kind)
            && current != target
        {
            self.move_page(current, target);
            self.mark_items_dirty();
        }
        true
    }

    pub(crate) fn handle_page_order_release(&mut self) -> bool {
        if self.page_order.drag.take().is_none() {
            return false;
        }
        self.persist_settings_change();
        true
    }

    pub(crate) fn page_order_animating(&self) -> bool {
        if !self.page_order_active() {
            return false;
        }
        let state = &self.page_order;
        let lift_target = f32::from(state.drag.is_some());
        (state.lift - lift_target).abs() > 0.001
            || state.drag.is_some()
            || state
                .press
                .is_some_and(|(_, _, started)| started.elapsed().as_secs_f32() < PRESS_SECS)
            || self
                .config
                .expanded_page_order
                .iter()
                .enumerate()
                .any(|(slot, kind)| (state.slots[kind_index(*kind)] - slot as f32).abs() > 0.001)
    }

    pub(crate) fn update_page_order_animation(&mut self, dt: f32) -> bool {
        if !self.page_order_active() {
            self.page_order.initialized = false;
            return false;
        }
        let order = self.config.expanded_page_order.clone();
        let dragged = self.page_order.drag.map(|drag| drag.kind);
        let state = &mut self.page_order;
        let mut changed = false;
        for (slot, kind) in order.iter().enumerate() {
            let value = &mut state.slots[kind_index(*kind)];
            if !state.initialized {
                *value = slot as f32;
                changed = true;
            } else if dragged != Some(*kind) {
                changed |= animate_towards(value, slot as f32, SLIDE_RATE, dt);
            }
        }
        state.initialized = true;
        changed |= animate_towards(&mut state.lift, f32::from(dragged.is_some()), LIFT_RATE, dt);
        if state
            .press
            .is_some_and(|(_, _, started)| started.elapsed().as_secs_f32() >= PRESS_SECS)
        {
            state.press = None;
            changed = true;
        }
        changed || state.press.is_some()
    }

    pub(crate) fn draw_page_order_list(&self, painter: Painter<'_>, theme: &SettingsTheme) {
        let Some(list) = self.page_order_list_rect() else {
            return;
        };
        let (window_width, window_height) = self.logical_window_size();
        let save_count = painter.save();
        painter.clip_rect(Rect::from_xywh(
            SIDEBAR_W,
            SETTINGS_HEADER_H,
            window_width - SIDEBAR_W,
            window_height - SETTINGS_HEADER_H,
        ));
        painter.fill_round_rect(
            list.offset(winisland_render::Vec2::new(0.0, 2.0)),
            Radius::uniform(GROUP_RADIUS),
            settings_color(theme.shadow),
        );
        painter.fill_round_rect(
            list,
            Radius::uniform(GROUP_RADIUS),
            settings_color(theme.group_bg),
        );
        painter.stroke_round_rect(
            list.inset(0.375),
            Radius::uniform(GROUP_RADIUS),
            0.75,
            settings_color(theme.group_border),
        );

        let order = self.config.expanded_page_order.clone();
        let count = order.len();
        for slot in 1..count {
            painter.fill_rect(
                Rect::from_xywh(
                    list.left + GROUP_INNER_PAD,
                    list.top + slot as f32 * ROW_HEIGHT,
                    list.width() - GROUP_INNER_PAD * 2.0,
                    0.75,
                ),
                settings_color(theme.separator),
            );
        }

        let dragged = self.page_order.drag;
        let pointer = Point::new(self.logical_mouse_pos.0, self.logical_mouse_pos.1);
        let mut rows: Vec<(usize, ExpandedPageKind, f32)> = order
            .iter()
            .enumerate()
            .map(|(slot, kind)| {
                let top = match dragged {
                    Some(drag) if drag.kind == *kind => {
                        (pointer.y - drag.grab_offset).clamp(list.top, list.bottom - ROW_HEIGHT)
                    }
                    _ => list.top + self.page_order.slots[kind_index(*kind)] * ROW_HEIGHT,
                };
                (slot, *kind, top)
            })
            .collect();
        rows.sort_by_key(|(_, kind, _)| dragged.is_some_and(|drag| drag.kind == *kind));

        for (slot, kind, top) in rows {
            let is_dragged = dragged.is_some_and(|drag| drag.kind == kind);
            let lift = if is_dragged {
                self.page_order.lift
            } else {
                0.0
            };
            let row = Rect::from_xywh(list.left, top, list.width(), ROW_HEIGHT);
            if lift > 0.001 {
                let grow = 3.0 * lift;
                let card = Rect::from_xywh(
                    row.left - grow,
                    row.top - grow,
                    row.width() + grow * 2.0,
                    row.height() + grow * 2.0,
                );
                painter.fill_round_rect(
                    card.offset(winisland_render::Vec2::new(0.0, 4.0 * lift)),
                    Radius::uniform(GROUP_RADIUS),
                    Rgba::from_argb((70.0 * lift) as u8, 0, 0, 0),
                );
                painter.fill_round_rect(
                    card,
                    Radius::uniform(GROUP_RADIUS),
                    settings_color(theme.card_highlight),
                );
            }

            let handle_x = row.left + GROUP_INNER_PAD;
            for line in 0..3 {
                painter.fill_round_rect(
                    Rect::from_xywh(
                        handle_x,
                        row.center_y() - 5.0 + line as f32 * 4.5,
                        14.0,
                        1.6,
                    ),
                    Radius::uniform(0.8),
                    settings_color(theme.text_sec).with_alpha(150),
                );
            }

            let fonts = FontManager::global();
            let name = tr(page_name_key(kind));
            let name_bounds = fonts.measure_str(&name, 14.0, false);
            let text_x = handle_x + 30.0;
            let baseline = row.center_y() - name_bounds.height() / 2.0 - name_bounds.top;
            fonts.draw_str(
                painter,
                &name,
                Point::new(text_x - name_bounds.left, baseline),
                14.0,
                false,
                settings_color(theme.text_pri),
            );
            let position = (slot + 1).to_string();
            fonts.draw_str(
                painter,
                &position,
                Point::new(text_x + name_bounds.width() + 10.0, baseline),
                12.0,
                false,
                settings_color(theme.text_sec),
            );

            let visual_row = (top - list.top) / ROW_HEIGHT;
            for direction in [ArrowDirection::Up, ArrowDirection::Down] {
                let mut button = Self::page_order_arrow_rect(list, 0, direction);
                button = button.offset(winisland_render::Vec2::new(0.0, visual_row * ROW_HEIGHT));
                let enabled = match direction {
                    ArrowDirection::Up => slot > 0,
                    ArrowDirection::Down => slot + 1 < count,
                };
                let hovered = enabled && dragged.is_none() && button.contains(pointer);
                let flash = self
                    .page_order
                    .press
                    .filter(|(pressed, pressed_direction, _)| {
                        *pressed == kind && *pressed_direction == direction
                    })
                    .map_or(0.0, |(_, _, started)| {
                        1.0 - (started.elapsed().as_secs_f32() / PRESS_SECS).min(1.0)
                    });
                let fill = if hovered || flash > 0.0 {
                    theme.control_hover
                } else {
                    theme.control_bg
                };
                let shrink = 1.5 * flash;
                painter.fill_round_rect(
                    button.inset(shrink),
                    Radius::uniform(7.0),
                    settings_color(fill),
                );
                let color = settings_color(if enabled {
                    theme.text_pri
                } else {
                    theme.disabled
                });
                let center = Point::new(button.center_x(), button.center_y());
                let sign = if direction == ArrowDirection::Up {
                    -1.0
                } else {
                    1.0
                };
                for side in [-1.0, 1.0] {
                    painter.stroke_line(
                        Point::new(center.x + side * 4.5, center.y - sign * 2.2),
                        Point::new(center.x, center.y + sign * 2.4),
                        1.7,
                        color,
                        StrokeCap::Round,
                    );
                }
            }
        }
        painter.restore_to(save_count);
    }
}
