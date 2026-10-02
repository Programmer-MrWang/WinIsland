use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low = 0,
    Medium = 1,
    High = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimerContent {
    pub remaining: Duration,
    pub total: Duration,
    pub paused: bool,
}

impl TimerContent {
    pub fn remaining_secs(self) -> u64 {
        self.remaining.as_secs() + u64::from(self.remaining.subsec_nanos() > 0)
    }

    pub fn progress(self) -> f32 {
        if self.total.is_zero() {
            return 1.0;
        }
        (1.0 - self.remaining.as_secs_f32() / self.total.as_secs_f32()).clamp(0.0, 1.0)
    }

    pub fn time_text(self) -> String {
        let remaining = self.remaining_secs();
        let hours = remaining / 3600;
        let minutes = remaining % 3600 / 60;
        let seconds = remaining % 60;
        if hours > 0 {
            format!("{hours}:{minutes:02}:{seconds:02}")
        } else {
            format!("{minutes}:{seconds:02}")
        }
    }
}

#[derive(Debug, Clone)]
pub struct PluginContext {
    pub id: u64,
    pub priority: Priority,
    pub title: String,
    pub body: String,
    pub compact_text: String,
    pub show_compact: bool,
    pub expires_at: Option<Instant>,
    pub updated_at: Instant,
}
