use std::cell::RefCell;
use std::sync::Arc;

use windows::Win32::Foundation::POINT;
use windows::Win32::Graphics::Gdi::{
    BeginPath, CloseFigure, CombineRgn, CreateCompatibleDC, CreateRectRgn, DeleteDC, DeleteObject,
    EndPath, HDC, HGDIOBJ, HRGN, LineTo, MoveToEx, PathToRegion, PolyBezierTo, RGN_ERROR, RGN_OR,
    SetPolyFillMode, SetWindowRgn, WINDING,
};
use winisland_platform::{ClipSegment, HitRegion, PlatformError, WindowSize};
use winit::window::Window;

pub(super) struct InputWindow {
    pub(super) window: Arc<Window>,
    regions: RefCell<Option<(WindowSize, Vec<HitRegion>)>>,
}

impl InputWindow {
    pub(super) fn new(window: Arc<Window>) -> Result<Self, PlatformError> {
        let input = Self {
            window,
            regions: RefCell::new(None),
        };
        input.set_regions(&[])?;
        Ok(input)
    }

    pub(super) fn sync(&self, owner: &Window) {
        if let Ok(position) = owner.inner_position()
            && self.window.outer_position().ok() != Some(position)
        {
            self.window.set_outer_position(position);
        }
        let size = owner.inner_size();
        if self.window.inner_size() != size {
            let _ = self.window.request_inner_size(size);
        }
        let visible = owner.is_visible().unwrap_or(false);
        if self.window.is_visible() != Some(visible) {
            self.window.set_visible(visible);
        }
    }

    pub(super) fn set_regions(&self, regions: &[HitRegion]) -> Result<(), PlatformError> {
        let native_size = self.window.inner_size();
        let size = WindowSize::new(native_size.width, native_size.height);
        if self
            .regions
            .borrow()
            .as_ref()
            .is_some_and(|(previous_size, previous)| *previous_size == size && previous == regions)
        {
            return Ok(());
        }
        let hwnd = super::window_hwnd(&self.window)
            .map(|hwnd| windows::Win32::Foundation::HWND(hwnd as *mut _))
            .ok_or(PlatformError::Unavailable("input window handle"))?;
        let combined =
            Region::rect(0, 0, 0, 0).ok_or(PlatformError::Unavailable("input window region"))?;
        for region in regions {
            let native = match region {
                HitRegion::WholeWindow(false) => continue,
                HitRegion::WholeWindow(true) => {
                    Region::rect(0, 0, size.width as i32, size.height as i32)
                }
                HitRegion::Rect(rect) => Region::rect(rect.left, rect.top, rect.right, rect.bottom),
                HitRegion::Path(segments) => Region::path(segments),
            }
            .ok_or(PlatformError::Unavailable("input window region geometry"))?;
            // SAFETY: All three region handles are live; GDI allows the destination to alias a source.
            if unsafe { CombineRgn(Some(combined.0), Some(combined.0), Some(native.0), RGN_OR) }
                == RGN_ERROR
            {
                return Err(PlatformError::Unavailable("input window region union"));
            }
        }
        // SAFETY: hwnd is live on this thread and combined owns the region until success transfers it.
        if unsafe { SetWindowRgn(hwnd, Some(combined.0), false) } == 0 {
            return Err(PlatformError::Unavailable("input window region assignment"));
        }
        std::mem::forget(combined);
        *self.regions.borrow_mut() = Some((size, regions.to_vec()));
        Ok(())
    }
}

struct Region(HRGN);

impl Region {
    fn rect(left: i32, top: i32, right: i32, bottom: i32) -> Option<Self> {
        // SAFETY: The coordinates are values; the returned region is owned by this wrapper.
        let region = unsafe { CreateRectRgn(left, top, right, bottom) };
        (!region.is_invalid()).then_some(Self(region))
    }

    fn path(segments: &[ClipSegment]) -> Option<Self> {
        let point = |[x, y]: [f32; 2]| POINT {
            x: x.round() as i32,
            y: y.round() as i32,
        };
        // SAFETY: This memory DC is owned locally and every path call receives initialized points.
        unsafe {
            let dc = MemoryDc(CreateCompatibleDC(None));
            if dc.0.is_invalid() || !BeginPath(dc.0).as_bool() {
                return None;
            }
            SetPolyFillMode(dc.0, WINDING);
            let mut current = [0.0; 2];
            let mut start = current;
            for segment in segments {
                let success = match *segment {
                    ClipSegment::Move(end) => {
                        current = end;
                        start = end;
                        let end = point(end);
                        MoveToEx(dc.0, end.x, end.y, None)
                    }
                    ClipSegment::Line(end) => {
                        current = end;
                        let end = point(end);
                        LineTo(dc.0, end.x, end.y)
                    }
                    ClipSegment::Quadratic(control, end) => {
                        let first = std::array::from_fn(|i| {
                            current[i] + (control[i] - current[i]) * 2.0 / 3.0
                        });
                        let second =
                            std::array::from_fn(|i| end[i] + (control[i] - end[i]) * 2.0 / 3.0);
                        current = end;
                        PolyBezierTo(dc.0, &[point(first), point(second), point(end)])
                    }
                    ClipSegment::Cubic(first, second, end) => {
                        current = end;
                        PolyBezierTo(dc.0, &[point(first), point(second), point(end)])
                    }
                    ClipSegment::Close => {
                        current = start;
                        CloseFigure(dc.0)
                    }
                };
                if !success.as_bool() {
                    return None;
                }
            }
            if !EndPath(dc.0).as_bool() {
                return None;
            }
            let region = PathToRegion(dc.0);
            (!region.is_invalid()).then_some(Self(region))
        }
    }
}

impl Drop for Region {
    fn drop(&mut self) {
        // SAFETY: The region has not been transferred to a window and is released exactly once.
        let _ = unsafe { DeleteObject(HGDIOBJ(self.0.0)) };
    }
}

struct MemoryDc(HDC);

impl Drop for MemoryDc {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            // SAFETY: The memory DC was created by CreateCompatibleDC and is owned by this wrapper.
            let _ = unsafe { DeleteDC(self.0) };
        }
    }
}

pub(super) fn set_hit_regions(window: &Window, regions: &[HitRegion]) {
    let enabled = regions
        .iter()
        .any(|region| matches!(region, HitRegion::WholeWindow(true)));
    let _ = window.set_cursor_hittest(enabled);
}
