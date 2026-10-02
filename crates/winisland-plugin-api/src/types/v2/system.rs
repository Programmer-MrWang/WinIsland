#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct LocalDateTimeV2 {
    pub struct_size: u32,
    pub year: u16,
    pub month: u16,
    pub day: u16,
    pub day_of_week: u16,
    pub hour: u16,
    pub minute: u16,
    pub second: u16,
    pub millisecond: u16,
}

impl Default for LocalDateTimeV2 {
    fn default() -> Self {
        Self {
            struct_size: std::mem::size_of::<Self>() as u32,
            year: 0,
            month: 0,
            day: 0,
            day_of_week: 0,
            hour: 0,
            minute: 0,
            second: 0,
            millisecond: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct LunarDateV2 {
    pub struct_size: u32,
    pub month: u8,
    pub day: u8,
    pub leap: u8,
    pub reserved: u8,
}

impl Default for LunarDateV2 {
    fn default() -> Self {
        Self {
            struct_size: std::mem::size_of::<Self>() as u32,
            month: 0,
            day: 0,
            leap: 0,
            reserved: 0,
        }
    }
}
