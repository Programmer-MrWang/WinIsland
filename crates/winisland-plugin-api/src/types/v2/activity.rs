use super::WidgetId;
pub use super::context::{PRIORITY_HIGH, PRIORITY_LOW, PRIORITY_MEDIUM};

pub const ACTIVITY_ENABLED: u32 = 1 << 0;
pub const ACTIVITY_KEEP_VISIBLE: u32 = 1 << 1;

#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct ActivitySpecV2 {
    pub struct_size: u32,
    pub flags: u32,
    pub priority: u32,
    pub timeout_ms: u32,
    pub compact_surface: WidgetId,
    pub expanded_page: WidgetId,
    pub preferred_width: f32,
    pub preferred_height: f32,
}

impl Default for ActivitySpecV2 {
    fn default() -> Self {
        Self {
            struct_size: std::mem::size_of::<Self>() as u32,
            flags: ACTIVITY_ENABLED,
            priority: PRIORITY_MEDIUM,
            timeout_ms: 0,
            compact_surface: WidgetId::INVALID,
            expanded_page: WidgetId::INVALID,
            preferred_width: 0.0,
            preferred_height: 0.0,
        }
    }
}
