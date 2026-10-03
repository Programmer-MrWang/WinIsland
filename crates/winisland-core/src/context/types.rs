use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low = 0,
    Medium = 1,
    High = 2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityText {
    pub title: String,
    pub body: String,
    pub compact_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivityContent {
    Local,
    Text(ActivityText),
    Surface(u64),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Activity {
    pub id: u64,
    pub priority: Priority,
    pub content: ActivityContent,
    pub show_compact: bool,
    pub keep_visible: bool,
    pub preferred_width: f32,
    pub preferred_height: f32,
    pub page: Option<u64>,
    pub expires_at: Option<Instant>,
    pub updated_at: Instant,
}

impl Activity {
    pub fn visible_at(&self, now: Instant) -> bool {
        self.show_compact && self.expires_at.is_none_or(|deadline| deadline > now)
    }
}
