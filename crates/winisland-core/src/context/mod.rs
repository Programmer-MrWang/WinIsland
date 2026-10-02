mod types;

pub use types::*;

use std::time::Instant;

#[derive(Debug, Clone, Copy)]
pub enum MiniContent<'a> {
    Music,
    Timer(TimerContent),
    Plugin(&'a PluginContext),
}

pub struct ContextManager {
    plugin_contexts: Vec<PluginContext>,
    smtc_active: bool,
    smtc_playing: bool,
    timer: Option<TimerContent>,
}

impl Default for ContextManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextManager {
    pub fn new() -> Self {
        Self {
            plugin_contexts: Vec::new(),
            smtc_active: false,
            smtc_playing: false,
            timer: None,
        }
    }

    pub fn set_smtc_active(&mut self, active: bool) {
        self.smtc_active = active;
    }

    pub fn set_smtc_state(&mut self, active: bool, playing: bool) -> bool {
        let playing = active && playing;
        let changed = self.smtc_active != active || self.smtc_playing != playing;
        self.smtc_active = active;
        self.smtc_playing = playing;
        changed
    }

    pub fn set_timer(&mut self, timer: Option<TimerContent>) -> bool {
        let display = |timer: TimerContent| (timer.remaining_secs(), timer.total, timer.paused);
        let changed = self.timer.map(display) != timer.map(display);
        self.timer = timer;
        changed
    }

    pub fn timer_active(&self) -> bool {
        self.timer.is_some()
    }

    pub fn upsert_context(&mut self, context: PluginContext) {
        if let Some(existing) = self
            .plugin_contexts
            .iter_mut()
            .find(|existing| existing.id == context.id)
        {
            *existing = context;
        } else {
            self.plugin_contexts.push(context);
        }
    }

    pub fn remove_context(&mut self, id: u64) -> bool {
        let original_len = self.plugin_contexts.len();
        self.plugin_contexts.retain(|context| context.id != id);
        self.plugin_contexts.len() != original_len
    }

    pub fn current_mini(&self) -> Option<MiniContent<'_>> {
        if let Some(context) = self
            .plugin_contexts
            .iter()
            .filter(|context| context.show_compact)
            .max_by_key(|context| (context.priority, context.updated_at))
        {
            return Some(MiniContent::Plugin(context));
        }
        let music = self.smtc_active.then_some((
            if self.smtc_playing {
                Priority::High
            } else {
                Priority::Low
            },
            MiniContent::Music,
        ));
        let timer = self
            .timer
            .map(|timer| (Priority::Medium, MiniContent::Timer(timer)));
        [music, timer]
            .into_iter()
            .flatten()
            .max_by_key(|(priority, _)| *priority)
            .map(|(_, content)| content)
    }

    pub fn tick(&mut self) -> bool {
        let now = Instant::now();
        let previous_len = self.plugin_contexts.len();
        self.plugin_contexts
            .retain(|context| context.expires_at.is_none_or(|expires_at| expires_at > now));
        self.plugin_contexts.len() != previous_len
    }
}
