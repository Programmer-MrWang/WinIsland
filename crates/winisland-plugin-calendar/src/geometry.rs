#[derive(Clone, Copy)]
pub(super) struct Point {
    pub(super) x: f32,
    pub(super) y: f32,
}

impl Point {
    pub(super) const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy)]
pub(super) struct Vec2 {
    pub(super) x: f32,
    pub(super) y: f32,
}

impl Vec2 {
    pub(super) const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy)]
pub(super) struct Rect {
    pub(super) left: f32,
    pub(super) top: f32,
    pub(super) right: f32,
    pub(super) bottom: f32,
}

impl Rect {
    pub(super) const fn from_xywh(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            left: x,
            top: y,
            right: x + width,
            bottom: y + height,
        }
    }

    pub(super) const fn width(&self) -> f32 {
        self.right - self.left
    }

    pub(super) const fn height(&self) -> f32 {
        self.bottom - self.top
    }

    pub(super) const fn center_x(&self) -> f32 {
        (self.left + self.right) * 0.5
    }

    pub(super) const fn center_y(&self) -> f32 {
        (self.top + self.bottom) * 0.5
    }

    pub(super) fn offset(self, delta: Vec2) -> Self {
        Self {
            left: self.left + delta.x,
            top: self.top + delta.y,
            right: self.right + delta.x,
            bottom: self.bottom + delta.y,
        }
    }

    pub(super) fn inset(self, delta: f32) -> Self {
        Self {
            left: self.left + delta,
            top: self.top + delta,
            right: self.right - delta,
            bottom: self.bottom - delta,
        }
    }
}
