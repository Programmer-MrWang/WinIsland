use std::collections::HashSet;

use winisland_plugin_api::*;

use super::{Event, Extensions, State};

#[derive(Default)]
pub struct InputOutcome {
    pub handled: bool,
    pub captured: bool,
    pub keyboard: bool,
}

impl State {
    pub(crate) fn release_capture(&mut self, target: u64) {
        if let Some(captured) = self.capture.filter(|captured| captured.0 == target) {
            self.capture = None;
            self.send_input(
                captured,
                Event {
                    detail: INPUT_CANCEL,
                    ..Default::default()
                },
            );
        }
    }

    fn accepts_input(&self, target: (u64, u64), flags: u32) -> bool {
        self.callbacks
            .values()
            .any(|callback| callback.target == target.0 && callback.mask & EVENT_INPUT != 0)
            && self.regions.get(&target.0).is_some_and(|regions| {
                regions
                    .iter()
                    .any(|region| region.id == target.1 && region.flags & flags != 0)
            })
    }

    fn send_input(&mut self, target: (u64, u64), mut event: Event) {
        event.kind = EVENT_INPUT;
        event.target = target.0;
        event.sequence = target.1;
        if let Some(p) = self.presentations.iter().find(|p| p.target == target.0) {
            event.x = (event.x - p.bounds[0]) * p.logical[0] / p.bounds[2].max(1.0);
            event.y = (event.y - p.bounds[1]) * p.logical[1] / p.bounds[3].max(1.0);
        }
        self.publish(None, event);
    }

    pub(super) fn cancel_missing(&mut self, active: &HashSet<u64>) {
        if self.capture.is_some_and(|target| {
            !active.contains(&target.0) || !self.accepts_input(target, INPUT_POINTER)
        }) && let Some(target) = self.capture.take()
        {
            self.send_input(
                target,
                Event {
                    detail: INPUT_CANCEL,
                    ..Default::default()
                },
            );
        }
        if self.focus.is_some_and(|target| {
            !active.contains(&target.0) || !self.accepts_input(target, INPUT_KEYBOARD)
        }) && let Some(target) = self.focus.take()
        {
            self.send_input(
                target,
                Event {
                    detail: INPUT_BLUR,
                    ..Default::default()
                },
            );
        }
        if self.hover.is_some_and(|target| {
            !active.contains(&target.0) || !self.accepts_input(target, INPUT_POINTER)
        }) && let Some(target) = self.hover.take()
        {
            self.send_input(
                target,
                Event {
                    detail: INPUT_LEAVE,
                    ..Default::default()
                },
            );
        }
    }
}

impl Extensions {
    pub fn input_state(&self) -> InputOutcome {
        self.state
            .lock()
            .map(|mut state| {
                let active = state
                    .presentations
                    .iter()
                    .filter(|p| p.interactive)
                    .map(|p| p.target)
                    .collect();
                state.cancel_missing(&active);
                InputOutcome {
                    handled: false,
                    captured: state.capture.is_some(),
                    keyboard: state.focus.is_some(),
                }
            })
            .unwrap_or_default()
    }

    pub fn route_input(&self, event: Event) -> InputOutcome {
        let Ok(mut state) = self.state.lock() else {
            return InputOutcome::default();
        };
        let active = state
            .presentations
            .iter()
            .filter(|p| p.interactive)
            .map(|p| p.target)
            .collect();
        state.cancel_missing(&active);
        if matches!(event.detail, INPUT_CANCEL | INPUT_BLUR) {
            state.cancel_missing(&HashSet::new());
            return InputOutcome::default();
        }
        let keyboard = matches!(
            event.detail,
            INPUT_KEY_DOWN | INPUT_KEY_UP | INPUT_TEXT | INPUT_COMPOSITION
        );
        let filter = match event.detail {
            INPUT_WHEEL => INPUT_SCROLL,
            INPUT_DROP => INPUT_FILES,
            _ => INPUT_POINTER,
        };
        let hit = state
            .presentations
            .iter()
            .rev()
            .filter(|p| p.interactive)
            .filter(|p| {
                state
                    .callbacks
                    .values()
                    .any(|callback| callback.target == p.target && callback.mask & EVENT_INPUT != 0)
            })
            .find_map(|p| {
                let x = (event.x - p.bounds[0]) * p.logical[0] / p.bounds[2].max(1.0);
                let y = (event.y - p.bounds[1]) * p.logical[1] / p.bounds[3].max(1.0);
                if x < 0.0 || y < 0.0 || x >= p.logical[0] || y >= p.logical[1] {
                    return None;
                }
                state
                    .regions
                    .get(&p.target)?
                    .iter()
                    .rev()
                    .find(|region| {
                        region.flags & filter != 0
                            && x >= region.x
                            && y >= region.y
                            && x < region.x + region.width
                            && y < region.y + region.height
                    })
                    .map(|region| ((p.target, region.id), region.flags))
            });
        if event.detail == INPUT_MOVE || event.detail == INPUT_LEAVE {
            let next = if event.detail == INPUT_LEAVE {
                None
            } else {
                hit.map(|hit| hit.0)
            };
            if state.hover != next {
                if let Some(old) = state.hover.take() {
                    state.send_input(
                        old,
                        Event {
                            detail: INPUT_LEAVE,
                            ..event.clone()
                        },
                    );
                }
                if let Some(next) = next {
                    state.send_input(
                        next,
                        Event {
                            detail: INPUT_ENTER,
                            ..event.clone()
                        },
                    );
                }
                state.hover = next;
            }
        }
        if event.detail == INPUT_LEAVE {
            return InputOutcome {
                handled: false,
                captured: state.capture.is_some(),
                keyboard: state.focus.is_some(),
            };
        }
        if event.detail == INPUT_DOWN && state.capture.is_none() {
            let next = hit
                .filter(|hit| hit.1 & INPUT_KEYBOARD != 0)
                .map(|hit| hit.0);
            if state.focus != next {
                if let Some(old) = state.focus.take() {
                    state.send_input(
                        old,
                        Event {
                            detail: INPUT_BLUR,
                            ..event.clone()
                        },
                    );
                }
                if let Some(next) = next {
                    state.send_input(
                        next,
                        Event {
                            detail: INPUT_FOCUS,
                            ..event.clone()
                        },
                    );
                }
                state.focus = next;
            }
            if let Some((target, flags)) = hit
                && flags & INPUT_CAPTURE_ON_PRESS != 0
            {
                state.capture = Some(target);
                state.capture_button = event.code;
            }
        }
        let target = if keyboard {
            state.focus
        } else if matches!(event.detail, INPUT_WHEEL | INPUT_DROP) {
            hit.map(|hit| hit.0)
        } else {
            state.capture.or(hit.map(|hit| hit.0))
        };
        if let Some(target) = target {
            state.send_input(target, event.clone());
        }
        if event.detail == INPUT_UP && event.code == state.capture_button {
            state.capture = None;
        }
        InputOutcome {
            handled: target.is_some(),
            captured: state.capture.is_some(),
            keyboard: state.focus.is_some(),
        }
    }
}
