use std::time::Instant;

use winisland_core::config::ExpandedPageKind;
use winisland_core::i18n::tr;
use winisland_render::text::FontManager;
use winisland_render::{Painter, Point, Radius, Rect, Rgba, StrokeCap};

use super::{SETTINGS_HEADER_H, SIDEBAR_W, SettingsApp, WIDGETS_PAGE_INDEX, animate_towards};
use crate::utils::color::SettingsTheme;
use crate::utils::settings_ui::input::{WIDGET_LIBRARY_TILE_H, widget_source_rect};
use crate::utils::settings_ui::items::{
    CONTENT_PADDING, GROUP_INNER_PAD, GROUP_RADIUS, ROW_HEIGHT, SettingsItem,
};
use crate::utils::settings_ui::renderer::{draw_delete_button, draw_library_tile_surface};
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
    library_width: Option<f32>,
}

pub(crate) struct PageOrderState {
    slots: [f32; ExpandedPageKind::ALL.len()],
    initialized: bool,
    drag: Option<PageOrderDrag>,
    lift: f32,
    press: Option<(ExpandedPageKind, ArrowDirection, Instant)>,
    original_order: Option<Vec<ExpandedPageKind>>,
    list_height: f32,
}

impl Default for PageOrderState {
    fn default() -> Self {
        Self {
            slots: [0.0; ExpandedPageKind::ALL.len()],
            initialized: false,
            drag: None,
            lift: 0.0,
            press: None,
            original_order: None,
            list_height: 0.0,
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
    count.max(1) as f32 * ROW_HEIGHT
}

pub(crate) fn page_library_height(count: usize, width: f32) -> f32 {
    if count == 0 {
        return ROW_HEIGHT + GROUP_INNER_PAD * 2.0;
    }
    let panel_width = width - CONTENT_PADDING * 2.0 - GROUP_INNER_PAD * 2.0;
    let (_, y, _, height) = widget_source_rect(0.0, panel_width, 0.0, count - 1);
    y + height + GROUP_INNER_PAD * 2.0
}

impl SettingsApp {
    pub(crate) fn page_order_display_height(&self) -> f32 {
        if self.page_order.initialized {
            self.page_order.list_height
        } else {
            page_order_list_height(self.config.expanded_page_order.len())
        }
    }

    fn page_order_active(&self) -> bool {
        self.active_page == WIDGETS_PAGE_INDEX
            && self.widget_editor_mode == WidgetEditorMode::Pages
            && !self.resource_editor_open
    }

    fn page_order_list_rect(&self) -> Option<Rect> {
        self.page_order_custom_rect(0)
    }

    fn page_order_library_rect(&self) -> Option<Rect> {
        self.page_order_custom_rect(1)
    }

    fn page_order_custom_rect(&self, index: usize) -> Option<Rect> {
        if !self.page_order_active() {
            return None;
        }
        let mut y = SETTINGS_HEADER_H;
        let mut custom = 0;
        for item in &self.cached_items {
            if let SettingsItem::Custom { height } = item {
                if custom == index {
                    return Some(Rect::from_xywh(
                        SIDEBAR_W + CONTENT_PADDING,
                        y - self.scroll_y,
                        self.content_width() - CONTENT_PADDING * 2.0,
                        *height,
                    ));
                }
                custom += 1;
            }
            y += item.height();
        }
        None
    }

    fn page_order_arrow_rect(list: Rect, row: usize, direction: ArrowDirection) -> Rect {
        let top = list.top + row as f32 * ROW_HEIGHT + (ROW_HEIGHT - ARROW_BUTTON) / 2.0;
        let right = list.right - GROUP_INNER_PAD - ARROW_BUTTON - ARROW_GAP;
        let left = match direction {
            ArrowDirection::Down => right - ARROW_BUTTON,
            ArrowDirection::Up => right - ARROW_BUTTON * 2.0 - ARROW_GAP,
        };
        Rect::from_xywh(left, top, ARROW_BUTTON, ARROW_BUTTON)
    }

    fn page_order_remove_rect(list: Rect, top: f32) -> Rect {
        Rect::from_xywh(
            list.right - GROUP_INNER_PAD - ARROW_BUTTON,
            top + (ROW_HEIGHT - ARROW_BUTTON) / 2.0,
            ARROW_BUTTON,
            ARROW_BUTTON,
        )
    }

    fn page_order_row_top(&self, list: Rect, slot: usize, kind: ExpandedPageKind) -> f32 {
        let position = if self.page_order.initialized {
            self.page_order.slots[kind_index(kind)]
        } else {
            slot as f32
        };
        list.top + position * ROW_HEIGHT
    }

    fn page_order_library_tiles(&self) -> Vec<(ExpandedPageKind, Rect)> {
        let Some(panel) = self.page_order_library_rect() else {
            return Vec::new();
        };
        ExpandedPageKind::ALL
            .into_iter()
            .filter(|kind| !self.config.expanded_page_order.contains(kind))
            .filter(|kind| self.page_order.drag.is_none_or(|drag| drag.kind != *kind))
            .enumerate()
            .map(|(index, kind)| {
                let (x, y, width, height) = widget_source_rect(
                    panel.left + GROUP_INNER_PAD,
                    panel.width() - GROUP_INNER_PAD * 2.0,
                    panel.top + GROUP_INNER_PAD,
                    index,
                );
                (kind, Rect::from_xywh(x, y, width, height))
            })
            .collect()
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
        self.config
            .expanded_page_order
            .iter()
            .enumerate()
            .rev()
            .find(|(slot, kind)| {
                Rect::from_xywh(
                    list.left,
                    self.page_order_row_top(list, *slot, **kind),
                    list.width(),
                    ROW_HEIGHT,
                )
                .contains(point)
            })
            .map(|(row, _)| (list, row))
    }

    pub(crate) fn page_order_hovered(&mut self, x: f32, y: f32) -> bool {
        self.page_order_active()
            && (self.page_order.drag.is_some()
                || self.page_order_row_at(x, y).is_some()
                || self
                    .page_order_library_tiles()
                    .iter()
                    .any(|(_, rect)| y >= SETTINGS_HEADER_H && rect.contains(Point::new(x, y))))
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
            if !self.page_order_active() || y < SETTINGS_HEADER_H {
                return false;
            }
            self.ensure_items_cache();
            let Some((kind, rect)) = self
                .page_order_library_tiles()
                .into_iter()
                .find(|(_, rect)| rect.contains(Point::new(x, y)))
            else {
                return false;
            };
            self.page_order.original_order = Some(self.config.expanded_page_order.clone());
            self.page_order.drag = Some(PageOrderDrag {
                kind,
                grab_offset: ROW_HEIGHT / 2.0,
                library_width: Some(rect.width()),
            });
            self.request_redraw();
            return true;
        };
        let count = self.config.expanded_page_order.len();
        let kind = self.config.expanded_page_order[row];
        let row_top = self.page_order_row_top(list, row, kind);
        if Self::page_order_remove_rect(list, row_top).contains(Point::new(x, y)) {
            self.config.expanded_page_order.remove(row);
            self.page_order.press = None;
            self.persist_settings_change();
            return true;
        }
        for direction in [ArrowDirection::Up, ArrowDirection::Down] {
            let button = Self::page_order_arrow_rect(list, 0, direction)
                .offset(winisland_render::Vec2::new(0.0, row_top - list.top));
            if !button.contains(Point::new(x, y)) {
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
        self.page_order.original_order = Some(self.config.expanded_page_order.clone());
        self.page_order.drag = Some(PageOrderDrag {
            kind,
            grab_offset: y - row_top,
            library_width: None,
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
        let pointer = Point::new(self.logical_mouse_pos.0, self.logical_mouse_pos.1);
        let inside = pointer.y >= SETTINGS_HEADER_H && list.contains(pointer);
        if !inside {
            if drag.library_width.is_some() && self.config.expanded_page_order.contains(&drag.kind)
            {
                self.config
                    .expanded_page_order
                    .retain(|kind| *kind != drag.kind);
                self.mark_items_dirty();
            }
            return true;
        }
        let pointer_slot = (self.logical_mouse_pos.1 - drag.grab_offset - list.top) / ROW_HEIGHT;
        let current = self
            .config
            .expanded_page_order
            .iter()
            .position(|kind| *kind == drag.kind);
        let maximum = if current.is_some() {
            count.saturating_sub(1)
        } else {
            count
        };
        let target = pointer_slot.round().clamp(0.0, maximum as f32) as usize;
        if let Some(current) = current {
            if current != target {
                self.move_page(current, target);
                self.mark_items_dirty();
            }
        } else {
            self.config.expanded_page_order.insert(target, drag.kind);
            self.page_order.slots[kind_index(drag.kind)] = target as f32;
            self.mark_items_dirty();
        }
        true
    }

    pub(crate) fn handle_page_order_release(&mut self) -> bool {
        if self.page_order.drag.is_none() {
            return false;
        }
        self.update_page_order_drag();
        self.ensure_items_cache();
        let pointer = Point::new(self.logical_mouse_pos.0, self.logical_mouse_pos.1);
        let inside = pointer.y >= SETTINGS_HEADER_H
            && self
                .page_order_list_rect()
                .is_some_and(|list| list.contains(pointer));
        if !inside {
            return self.cancel_page_order_drag();
        }
        self.page_order.drag = None;
        let changed = self
            .page_order
            .original_order
            .take()
            .is_some_and(|original| original != self.config.expanded_page_order);
        if changed {
            self.persist_settings_change();
        } else {
            self.mark_items_dirty();
        }
        true
    }

    pub(crate) fn cancel_page_order_drag(&mut self) -> bool {
        if self.page_order.drag.take().is_none() {
            return false;
        }
        if let Some(original) = self.page_order.original_order.take() {
            self.config.expanded_page_order = original;
        }
        self.mark_items_dirty();
        self.request_redraw();
        true
    }

    pub(crate) fn page_order_animating(&self) -> bool {
        if !self.page_order_active() {
            return false;
        }
        let state = &self.page_order;
        let lift_target = f32::from(state.drag.is_some());
        (state.lift - lift_target).abs() > 0.001
            || (state.list_height - page_order_list_height(self.config.expanded_page_order.len()))
                .abs()
                > 0.001
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
        let target_height = page_order_list_height(order.len());
        let layout_changed = if state.initialized {
            animate_towards(&mut state.list_height, target_height, SLIDE_RATE, dt)
        } else {
            state.list_height = target_height;
            true
        };
        changed |= layout_changed;
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
        let animating = changed || state.press.is_some();
        if layout_changed {
            self.mark_items_dirty();
        }
        animating
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
        self.draw_page_order_library(painter, theme, pointer);
        if order.is_empty() {
            Self::draw_page_order_label(painter, list, &tr("page_order_empty"), theme);
        }
        let mut rows: Vec<(usize, ExpandedPageKind, f32)> = order
            .iter()
            .enumerate()
            .map(|(slot, kind)| {
                let top = match dragged {
                    Some(drag) if drag.kind == *kind => pointer.y - drag.grab_offset,
                    _ => list.top + self.page_order.slots[kind_index(*kind)] * ROW_HEIGHT,
                };
                (slot, *kind, top)
            })
            .collect();
        if let Some(drag) = dragged
            && !order.contains(&drag.kind)
        {
            if let Some(width) = drag.library_width {
                let tile = Rect::from_xywh(
                    pointer.x - width / 2.0,
                    pointer.y - WIDGET_LIBRARY_TILE_H / 2.0,
                    width,
                    WIDGET_LIBRARY_TILE_H,
                );
                draw_library_tile_surface(painter, tile, self.page_order.lift, theme);
                Self::draw_page_order_label(painter, tile, &tr(page_name_key(drag.kind)), theme);
            } else {
                rows.push((count, drag.kind, pointer.y - drag.grab_offset));
            }
        }
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
            let position = if slot < count {
                (slot + 1).to_string()
            } else {
                String::new()
            };
            fonts.draw_str(
                painter,
                &position,
                Point::new(text_x + name_bounds.width() + 10.0, baseline),
                12.0,
                false,
                settings_color(theme.text_sec),
            );

            let visual_row = (top - list.top) / ROW_HEIGHT;
            if is_dragged {
                continue;
            }
            let remove = Self::page_order_remove_rect(list, top);
            draw_delete_button(
                painter,
                remove.center_x(),
                remove.center_y(),
                if remove.contains(pointer) { 1.12 } else { 1.0 },
            );
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

    fn draw_page_order_label(painter: Painter<'_>, rect: Rect, text: &str, theme: &SettingsTheme) {
        let fonts = FontManager::global();
        let bounds = fonts.measure_str(text, 13.0, false);
        fonts.draw_str(
            painter,
            text,
            Point::new(
                rect.center_x() - bounds.width() / 2.0 - bounds.left,
                rect.center_y() - bounds.height() / 2.0 - bounds.top,
            ),
            13.0,
            false,
            settings_color(theme.text_sec),
        );
    }

    fn draw_page_order_library(&self, painter: Painter<'_>, theme: &SettingsTheme, pointer: Point) {
        let Some(panel) = self.page_order_library_rect() else {
            return;
        };
        painter.fill_round_rect(
            panel,
            Radius::uniform(GROUP_RADIUS),
            settings_color(theme.group_bg),
        );
        painter.stroke_round_rect(
            panel.inset(0.375),
            Radius::uniform(GROUP_RADIUS),
            0.75,
            settings_color(theme.group_border),
        );
        let tiles = self.page_order_library_tiles();
        if tiles.is_empty() && self.page_order.drag.is_none() {
            Self::draw_page_order_label(painter, panel, &tr("page_library_empty"), theme);
        }
        for (kind, rect) in tiles {
            let hover = f32::from(self.page_order.drag.is_none() && rect.contains(pointer));
            draw_library_tile_surface(painter, rect, hover, theme);
            Self::draw_page_order_label(painter, rect, &tr(page_name_key(kind)), theme);
        }
    }
}
