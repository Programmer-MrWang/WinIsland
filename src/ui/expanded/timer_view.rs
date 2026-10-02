use std::cell::RefCell;
use std::time::{Duration, Instant};

use crate::ui::rolling_time::{RollingTime, TimeAnchor};
use crate::ui::widget::expanded::{draw_widget_text_centered, widget_grid_layout};
use winisland_core::context::TimerContent;
use winisland_core::i18n::{tr, tr_args};
use winisland_render::text::FontManager;
use winisland_render::{Angle, Painter, Point, Radius, Rect, Rgba, StrokeCap};

const ORANGE: Rgba = Rgba::from_rgb(255, 159, 10);
const GREEN: Rgba = Rgba::from_rgb(48, 209, 88);
const PRESETS: [u64; 6] = [1, 5, 10, 15, 25, 60];
const DEFAULT_MINUTES: u64 = 5;
const COLUMN_GAP: f32 = 18.0;
const CHIP_GAP: f32 = 6.0;
const PRESS_SECS: f32 = 0.26;
const HOVER_RATE: f32 = 18.0;
const EASE_RATE: f32 = 9.0;
const LABEL_SECS: f32 = 0.22;
const POP_SECS: f32 = 0.5;
const FINISHED_PULSE_SECS: f32 = 8.0;
const SETTLE_EPSILON: f32 = 0.002;
const CONTROL_COUNT: usize = PRESETS.len() + 3;
const MIN_MINUTES: u64 = 1;
const MAX_MINUTES: u64 = 180;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TimerAction {
    Preset(usize),
    Primary,
    Cancel,
    Dial,
}

impl TimerAction {
    fn index(self) -> usize {
        match self {
            Self::Preset(index) => index,
            Self::Primary => PRESETS.len(),
            Self::Cancel => PRESETS.len() + 1,
            Self::Dial => PRESETS.len() + 2,
        }
    }
}

#[derive(Clone, Copy)]
enum Phase {
    Idle,
    Running { ends_at: Instant },
    Paused { remaining: Duration },
    Finished { at: Instant },
}

#[derive(Clone, Copy, PartialEq)]
enum PrimaryStyle {
    Start,
    Pause,
    Resume,
}

impl PrimaryStyle {
    fn of(phase: Phase) -> Self {
        match phase {
            Phase::Idle | Phase::Finished { .. } => Self::Start,
            Phase::Running { .. } => Self::Pause,
            Phase::Paused { .. } => Self::Resume,
        }
    }

    fn color(self) -> Rgba {
        match self {
            Self::Start | Self::Resume => GREEN,
            Self::Pause => ORANGE,
        }
    }

    fn label(self) -> String {
        tr(match self {
            Self::Start => "timer_start",
            Self::Pause => "timer_pause",
            Self::Resume => "timer_resume",
        })
    }
}

struct TimerState {
    minutes: u64,
    phase: Phase,
    hover_target: Option<TimerAction>,
    hover: [f32; CONTROL_COUNT],
    press: Option<(TimerAction, Instant)>,
    last_frame: Option<Instant>,
    ring_fraction: f32,
    ring_alpha: f32,
    chips_enabled: f32,
    time: RollingTime,
    primary: PrimaryStyle,
    primary_change: Option<(PrimaryStyle, Instant)>,
}

thread_local! {
    static TIMER: RefCell<TimerState> = const { RefCell::new(TimerState {
        minutes: DEFAULT_MINUTES,
        phase: Phase::Idle,
        hover_target: None,
        hover: [0.0; CONTROL_COUNT],
        press: None,
        last_frame: None,
        ring_fraction: 1.0,
        ring_alpha: 0.35,
        chips_enabled: 1.0,
        time: RollingTime::new(),
        primary: PrimaryStyle::Start,
        primary_change: None,
    }) };
}

struct TimerLayout {
    ring_center: Point,
    ring_radius: f32,
    chips: [Rect; PRESETS.len()],
    primary: Rect,
    cancel: Rect,
}

fn layout(ox: f32, oy: f32, w: f32, h: f32, scale: f32) -> TimerLayout {
    let grid_layout = widget_grid_layout(ox, oy, w, h, scale);
    let (inner_x, inner_y, _, _) = grid_layout.slot_rect(0);
    let inset_x = inner_x - ox;
    let inset_y = inner_y - oy;
    let inner = Rect::from_xywh(
        inner_x,
        inner_y,
        (w - inset_x * 2.0).max(0.0),
        (h - inset_y * 2.0).max(0.0),
    );
    let diameter = inner.height().min(inner.width() * 0.48);
    let ring_center = Point::new(inner.left + diameter / 2.0, inner.center_y());
    let column_top = ring_center.y - diameter / 2.0;
    let column_bottom = ring_center.y + diameter / 2.0;
    let column_left = inner.left + diameter + COLUMN_GAP * scale;
    let column_w = (inner.right - column_left).max(0.0);
    let chip_gap = CHIP_GAP * scale;
    let chip_h = (diameter * 0.2).clamp(14.0 * scale, 24.0 * scale);
    let chip_w = ((column_w - chip_gap * 2.0) / 3.0).max(0.0);
    let chips = std::array::from_fn(|index| {
        let column = (index % 3) as f32;
        let row = (index / 3) as f32;
        Rect::from_xywh(
            column_left + column * (chip_w + chip_gap),
            column_top + row * (chip_h + chip_gap),
            chip_w,
            chip_h,
        )
    });
    let buttons_top = column_top + (chip_h + chip_gap) * 2.0;
    let button_h = (column_bottom - buttons_top).max(0.0);
    let button_w = ((column_w - chip_gap) / 2.0).max(0.0);
    TimerLayout {
        ring_center,
        ring_radius: diameter / 2.0,
        chips,
        cancel: Rect::from_xywh(column_left, buttons_top, button_w, button_h),
        primary: Rect::from_xywh(
            column_left + button_w + chip_gap,
            buttons_top,
            button_w,
            button_h,
        ),
    }
}

fn progress(started: Instant, now: Instant, duration: f32) -> Option<f32> {
    let t = now.saturating_duration_since(started).as_secs_f32() / duration;
    (t < 1.0).then_some(t.max(0.0))
}

fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

fn ease_out_back(t: f32) -> f32 {
    const OVERSHOOT: f32 = 1.70158;
    let shifted = t - 1.0;
    1.0 + (OVERSHOOT + 1.0) * shifted.powi(3) + OVERSHOOT * shifted.powi(2)
}

fn approach(value: &mut f32, target: f32, blend: f32) {
    *value += (target - *value) * blend;
    if (target - *value).abs() <= SETTLE_EPSILON {
        *value = target;
    }
}

fn presets_enabled(phase: Phase) -> bool {
    matches!(phase, Phase::Idle | Phase::Finished { .. })
}

fn total_duration(minutes: u64) -> Duration {
    Duration::from_secs(minutes * 60)
}

fn remaining(phase: Phase, minutes: u64, now: Instant) -> Duration {
    match phase {
        Phase::Idle => total_duration(minutes),
        Phase::Running { ends_at } => ends_at.saturating_duration_since(now),
        Phase::Paused { remaining } => remaining,
        Phase::Finished { .. } => Duration::ZERO,
    }
}

fn ring_targets(phase: Phase, minutes: u64, now: Instant) -> (f32, f32) {
    let total = total_duration(minutes);
    let fraction = if total.is_zero() {
        0.0
    } else {
        (remaining(phase, minutes, now).as_secs_f32() / total.as_secs_f32()).clamp(0.0, 1.0)
    };
    match phase {
        Phase::Idle => (1.0, 0.35),
        Phase::Running { .. } => (fraction, 1.0),
        Phase::Paused { .. } => (fraction, 0.55),
        Phase::Finished { .. } => (1.0, 1.0),
    }
}

pub fn apply_action(action: TimerAction) {
    TIMER.with(|cell| {
        let mut timer = cell.borrow_mut();
        let now = Instant::now();
        timer.press = Some((action, now));
        let total = total_duration(timer.minutes);
        timer.phase = match (action, timer.phase) {
            (TimerAction::Preset(index), phase) if presets_enabled(phase) => {
                timer.minutes = PRESETS[index.min(PRESETS.len() - 1)];
                Phase::Idle
            }
            (TimerAction::Primary, Phase::Idle | Phase::Finished { .. }) => Phase::Running {
                ends_at: now + total,
            },
            (TimerAction::Primary, Phase::Running { ends_at }) => Phase::Paused {
                remaining: ends_at.saturating_duration_since(now),
            },
            (TimerAction::Primary, Phase::Paused { remaining }) => Phase::Running {
                ends_at: now + remaining,
            },
            (TimerAction::Cancel, _) => Phase::Idle,
            (_, phase) => phase,
        };
    });
}

pub fn poll_finished() -> Option<u64> {
    TIMER.with(|cell| {
        let mut timer = cell.borrow_mut();
        let now = Instant::now();
        let finished = match timer.phase {
            Phase::Running { ends_at } => now >= ends_at,
            Phase::Paused { remaining } => remaining.is_zero(),
            Phase::Idle | Phase::Finished { .. } => false,
        };
        if !finished {
            return None;
        }
        timer.phase = Phase::Finished { at: now };
        Some(timer.minutes)
    })
}

pub fn compact_content() -> Option<TimerContent> {
    TIMER.with(|cell| {
        let timer = cell.borrow();
        if !matches!(timer.phase, Phase::Running { .. } | Phase::Paused { .. }) {
            return None;
        }
        let left = remaining(timer.phase, timer.minutes, Instant::now());
        Some(TimerContent {
            remaining: left,
            total: total_duration(timer.minutes),
            paused: matches!(timer.phase, Phase::Paused { .. }),
        })
    })
}

pub fn set_hover(action: Option<TimerAction>) -> bool {
    TIMER.with(|cell| {
        let mut timer = cell.borrow_mut();
        let changed = timer.hover_target != action;
        timer.hover_target = action;
        changed
    })
}

pub fn needs_frames() -> bool {
    TIMER.with(|cell| {
        let timer = cell.borrow();
        let now = Instant::now();
        let (ring_fraction, ring_alpha) = ring_targets(timer.phase, timer.minutes, now);
        let chips_target = if presets_enabled(timer.phase) {
            1.0
        } else {
            0.35
        };
        matches!(timer.phase, Phase::Running { .. })
            || matches!(timer.phase, Phase::Finished { at } if progress(at, now, FINISHED_PULSE_SECS).is_some())
            || timer
                .press
                .is_some_and(|(_, started)| progress(started, now, PRESS_SECS).is_some())
            || timer
                .primary_change
                .is_some_and(|(_, started)| progress(started, now, LABEL_SECS).is_some())
            || timer.time.is_animating()
            || (timer.ring_fraction - ring_fraction).abs() > SETTLE_EPSILON
            || (timer.ring_alpha - ring_alpha).abs() > SETTLE_EPSILON
            || (timer.chips_enabled - chips_target).abs() > SETTLE_EPSILON
            || timer.hover.iter().enumerate().any(|(index, value)| {
                let target = f32::from(
                    timer
                        .hover_target
                        .is_some_and(|action| action.index() == index),
                );
                (target - value).abs() > SETTLE_EPSILON
            })
    })
}

pub fn hit_test(ox: f32, oy: f32, w: f32, h: f32, scale: f32, point: Point) -> Option<TimerAction> {
    let layout = layout(ox, oy, w, h, scale);
    let phase = TIMER.with(|cell| cell.borrow().phase);
    if layout.primary.contains(point) {
        return Some(TimerAction::Primary);
    }
    if !matches!(phase, Phase::Idle) && layout.cancel.contains(point) {
        return Some(TimerAction::Cancel);
    }
    if !presets_enabled(phase) {
        return None;
    }
    let dx = point.x - layout.ring_center.x;
    let dy = point.y - layout.ring_center.y;
    if dx * dx + dy * dy <= layout.ring_radius * layout.ring_radius {
        return Some(TimerAction::Dial);
    }
    layout
        .chips
        .iter()
        .position(|chip| chip.contains(point))
        .map(TimerAction::Preset)
}

pub fn adjust_minutes(steps: i32) -> bool {
    TIMER.with(|cell| {
        let mut timer = cell.borrow_mut();
        if steps == 0 || !presets_enabled(timer.phase) {
            return false;
        }
        let mut minutes = timer.minutes;
        for _ in 0..steps.unsigned_abs() {
            minutes = if steps > 0 {
                minutes + if minutes < 60 { 1 } else { 5 }
            } else {
                minutes.saturating_sub(if minutes <= 60 { 1 } else { 5 })
            }
            .clamp(MIN_MINUTES, MAX_MINUTES);
        }
        if minutes == timer.minutes {
            return false;
        }
        timer.minutes = minutes;
        timer.phase = Phase::Idle;
        true
    })
}

fn format_remaining(duration: Duration) -> String {
    let total = duration.as_secs() + u64::from(duration.subsec_nanos() > 0);
    let (hours, minutes, seconds) = (total / 3600, total % 3600 / 60, total % 60);
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

fn end_time_text(remaining: Duration) -> String {
    let now = crate::platform::shell().local_datetime();
    let seconds_today = u64::from(now.hour) * 3600
        + u64::from(now.minute) * 60
        + u64::from(now.second)
        + remaining.as_secs();
    let minutes = (seconds_today / 60) % (24 * 60);
    format!("{:02}:{:02}", minutes / 60, minutes % 60)
}

fn fitted_size(text: &str, size: f32, max_width: f32) -> f32 {
    let width = FontManager::global().measure_str(text, size, true).width();
    if width <= max_width || width <= f32::EPSILON {
        size
    } else {
        size * max_width / width
    }
}

struct Frame {
    minutes: u64,
    phase: Phase,
    hover: [f32; CONTROL_COUNT],
    press: Option<(TimerAction, f32)>,
    ring_fraction: f32,
    ring_alpha: f32,
    chips_enabled: f32,
    primary: PrimaryStyle,
    primary_change: Option<(PrimaryStyle, f32)>,
}

fn advance(now: Instant) -> Frame {
    TIMER.with(|cell| {
        let mut timer = cell.borrow_mut();
        let dt = timer
            .last_frame
            .map_or(0.0, |last| {
                now.saturating_duration_since(last).as_secs_f32()
            })
            .min(0.1);
        timer.last_frame = Some(now);
        let hover_blend = 1.0 - (-dt * HOVER_RATE).exp();
        let ease_blend = 1.0 - (-dt * EASE_RATE).exp();
        let hover_target = timer.hover_target;
        for (index, value) in timer.hover.iter_mut().enumerate() {
            let target = f32::from(hover_target.is_some_and(|action| action.index() == index));
            approach(value, target, hover_blend);
        }
        let (ring_fraction, ring_alpha) = ring_targets(timer.phase, timer.minutes, now);
        let running = matches!(timer.phase, Phase::Running { .. });
        let fraction_blend = if running && (timer.ring_fraction - ring_fraction).abs() < 0.02 {
            1.0
        } else {
            ease_blend
        };
        approach(&mut timer.ring_fraction, ring_fraction, fraction_blend);
        approach(&mut timer.ring_alpha, ring_alpha, ease_blend);
        let chips_target = if presets_enabled(timer.phase) {
            1.0
        } else {
            0.35
        };
        approach(&mut timer.chips_enabled, chips_target, ease_blend);

        let style = PrimaryStyle::of(timer.phase);
        if style != timer.primary {
            timer.primary_change = Some((timer.primary, now));
            timer.primary = style;
        }

        let press = timer.press.and_then(|(action, started)| {
            progress(started, now, PRESS_SECS).map(|t| (action, 1.0 - t))
        });
        if press.is_none() {
            timer.press = None;
        }
        let primary_change = timer.primary_change.and_then(|(previous, started)| {
            progress(started, now, LABEL_SECS).map(|t| (previous, ease_out_cubic(t)))
        });
        if primary_change.is_none() {
            timer.primary_change = None;
        }
        Frame {
            minutes: timer.minutes,
            phase: timer.phase,
            hover: timer.hover,
            press,
            ring_fraction: timer.ring_fraction,
            ring_alpha: timer.ring_alpha,
            chips_enabled: timer.chips_enabled,
            primary: timer.primary,
            primary_change,
        }
    })
}

fn blend_color(from: Rgba, to: Rgba, amount: f32) -> Rgba {
    let mix = |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * amount) as u8;
    Rgba::from_rgb(
        mix(from.r(), to.r()),
        mix(from.g(), to.g()),
        mix(from.b(), to.b()),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn draw_timer_page(
    painter: Painter<'_>,
    ox: f32,
    oy: f32,
    w: f32,
    h: f32,
    alpha: u8,
    scale: f32,
    text_color: Rgba,
) {
    if alpha <= 20 {
        return;
    }
    let layout = layout(ox, oy, w, h, scale);
    let opacity = f32::from(alpha) / 255.0;
    let now = Instant::now();
    let (minutes, phase) = TIMER.with(|cell| {
        let timer = cell.borrow();
        (timer.minutes, timer.phase)
    });
    let left = remaining(phase, minutes, now);
    let time_text = format_remaining(left);
    let frame = advance(now);
    let pressed = |action: TimerAction| {
        frame
            .press
            .filter(|(pressed, _)| *pressed == action)
            .map_or(0.0, |(_, amount)| amount)
    };

    let center = layout.ring_center;
    let radius = layout.ring_radius;
    let stroke = (radius * 0.12).max(3.0 * scale);
    let ring_radius = (radius - stroke / 2.0).max(0.0);
    painter.stroke_circle(
        center,
        ring_radius,
        stroke,
        text_color.with_alpha_f((0.12 + 0.08 * frame.hover[TimerAction::Dial.index()]) * opacity),
    );
    let pulse = match frame.phase {
        Phase::Finished { at } => progress(at, now, FINISHED_PULSE_SECS).map_or(1.0, |t| {
            0.6 + 0.4 * (t * FINISHED_PULSE_SECS * std::f32::consts::TAU / 1.2).cos()
        }),
        _ => 1.0,
    };
    if frame.ring_fraction > 0.001 {
        painter.stroke_arc(
            Rect::from_xywh(
                center.x - ring_radius,
                center.y - ring_radius,
                ring_radius * 2.0,
                ring_radius * 2.0,
            ),
            Angle::ZERO,
            Angle::from_degrees(frame.ring_fraction * 360.0),
            stroke,
            ORANGE.with_alpha_f(frame.ring_alpha * pulse * opacity),
            StrokeCap::Round,
        );
    }

    let inner_width = (ring_radius - stroke) * 1.7;
    let pop = match frame.phase {
        Phase::Finished { at } => {
            progress(at, now, POP_SECS).map_or(1.0, |t| 0.9 + 0.1 * ease_out_back(t))
        }
        _ => 1.0,
    };
    let time_size = fitted_size(&time_text, radius * 0.42, inner_width) * pop;
    TIMER.with(|cell| {
        cell.borrow_mut().time.draw(
            painter,
            &time_text,
            TimeAnchor::Center(Point::new(center.x, center.y - time_size * 0.12)),
            time_size,
            text_color.with_alpha_f(opacity),
        );
    });
    let dial_hover = frame.hover[TimerAction::Dial.index()];
    let (caption, caption_color) = match frame.phase {
        Phase::Idle | Phase::Finished { .. } if dial_hover > 0.5 => {
            (tr("timer_scroll_hint"), ORANGE.with_alpha_f(0.85 * opacity))
        }
        Phase::Idle => (tr("timer_title"), text_color.with_alpha_f(0.55 * opacity)),
        Phase::Running { .. } => (
            tr_args("timer_ends_at", &[&end_time_text(left)]),
            text_color.with_alpha_f(0.55 * opacity),
        ),
        Phase::Paused { .. } => (tr("timer_pause"), ORANGE.with_alpha_f(0.85 * opacity)),
        Phase::Finished { .. } => (tr("timer_done"), ORANGE.with_alpha_f(opacity)),
    };
    let caption_size = fitted_size(&caption, (radius * 0.16).max(8.0 * scale), inner_width);
    draw_widget_text_centered(
        painter,
        &caption,
        Rect::from_xywh(
            center.x - radius,
            center.y + time_size * 0.48,
            radius * 2.0,
            caption_size * 1.4,
        ),
        caption_size,
        true,
        caption_color,
    );

    for (index, chip) in layout.chips.iter().enumerate() {
        if chip.height() <= 1.0 {
            continue;
        }
        let action = TimerAction::Preset(index);
        let selected = PRESETS[index] == frame.minutes;
        let enabled_factor = frame.chips_enabled;
        let hover_amount = frame.hover[action.index()];
        let fill = if selected {
            ORANGE.with_alpha_f((0.24 + 0.08 * hover_amount) * enabled_factor * opacity)
        } else {
            text_color.with_alpha_f(
                (0.08 + 0.06 * hover_amount + 0.12 * pressed(action)) * enabled_factor * opacity,
            )
        };
        painter.fill_round_rect(*chip, Radius::uniform(chip.height() / 2.0), fill);
        let label = tr_args("timer_minutes", &[&PRESETS[index].to_string()]);
        let label_size = fitted_size(
            &label,
            chip.height() * 0.5,
            chip.width() - chip.height() * 0.5,
        );
        draw_widget_text_centered(
            painter,
            &label,
            *chip,
            label_size,
            true,
            if selected {
                ORANGE.with_alpha_f(enabled_factor * opacity)
            } else {
                text_color.with_alpha_f(0.85 * enabled_factor * opacity)
            },
        );
    }

    let cancel_enabled = if matches!(frame.phase, Phase::Idle) {
        0.4
    } else {
        1.0
    };
    let (primary_color, primary_labels) = match frame.primary_change {
        Some((previous, eased)) => (
            blend_color(previous.color(), frame.primary.color(), eased),
            vec![
                (previous.label(), 1.0 - eased),
                (frame.primary.label(), eased),
            ],
        ),
        None => (frame.primary.color(), vec![(frame.primary.label(), 1.0)]),
    };
    let buttons = [
        (
            TimerAction::Cancel,
            layout.cancel,
            vec![(tr("timer_cancel"), 1.0)],
            text_color,
            cancel_enabled,
        ),
        (
            TimerAction::Primary,
            layout.primary,
            primary_labels,
            primary_color,
            1.0,
        ),
    ];
    for (action, rect, labels, tint, enabled_factor) in buttons {
        if rect.width() <= 1.0 {
            continue;
        }
        let hover_amount = frame.hover[action.index()];
        let press_amount = pressed(action);
        let inset = rect.height().min(rect.width()) * 0.03 * press_amount;
        let body = rect.inset(inset);
        let corner = Radius::uniform(body.height().min(body.width()) / 2.0);
        let fill_alpha = if action == TimerAction::Cancel {
            0.16 + 0.06 * hover_amount + 0.12 * press_amount
        } else {
            0.24 + 0.08 * hover_amount + 0.14 * press_amount
        };
        painter.fill_round_rect(
            body,
            corner,
            tint.with_alpha_f(fill_alpha * enabled_factor * opacity),
        );
        painter.stroke_round_rect(
            body.inset(0.5 * scale),
            corner,
            scale,
            tint.with_alpha_f(0.16 * enabled_factor * opacity),
        );
        let label_alpha = if action == TimerAction::Cancel {
            0.9
        } else {
            1.0
        };
        for (label, weight) in labels {
            let label_size = fitted_size(
                &label,
                (body.height() * 0.36).min(15.0 * scale),
                body.width() - body.height() * 0.5,
            );
            draw_widget_text_centered(
                painter,
                &label,
                rect,
                label_size,
                true,
                tint.with_alpha_f(label_alpha * weight * enabled_factor * opacity),
            );
        }
    }
}
