use winisland_plugin_api::sdk::*;
use winisland_plugin_api::{
    ACTIVITY_ENABLED, ACTIVITY_KEEP_VISIBLE, ActivitySpecV2, PRIORITY_HIGH, PRIORITY_MEDIUM,
    SURFACE_COMPACT_MAIN, SURFACE_ENABLED, SURFACE_PAGE, SurfaceSpecV2,
};

struct Download {
    activity: Activity,
    compact: Surface,
    page: Surface,
    spec: ActivitySpecV2,
}

impl Download {
    fn new(host: &Host) -> Result<Self, Error> {
        let surfaces = host.surfaces()?;
        let compact = surfaces.create(surface_spec(
            "download-compact",
            SURFACE_COMPACT_MAIN,
            220.0,
            36.0,
        ))?;
        let page = surfaces.create(surface_spec("download-page", SURFACE_PAGE, 360.0, 200.0))?;
        let spec = ActivitySpecV2 {
            flags: ACTIVITY_ENABLED | ACTIVITY_KEEP_VISIBLE,
            compact_surface: compact.id(),
            expanded_page: page.id(),
            ..Default::default()
        };
        let activity = host.activities()?.create(spec)?;
        Ok(Self {
            activity,
            compact,
            page,
            spec,
        })
    }

    fn on_progress(&mut self, percent: u8) -> Result<(), Error> {
        let percent = percent.min(100);
        let title = format!("Download {percent}%");
        for surface in [&self.compact, &self.page] {
            let (w, h) = surface.logical_size();
            let mut list = DrawListBuilder::new(Size::new(w, h));
            list.fill_round_rect(
                Rect::new(8.0, h - 8.0, (w - 16.0) * f32::from(percent) / 100.0, 3.0),
                1.5,
                Rgba::WHITE,
            );
            list.text(
                &title,
                Rect::new(12.0, 4.0, w - 24.0, h - 16.0),
                &TextStyle::default_at(13.0),
                Rgba::WHITE,
            );
            surface.submit(list.finish())?;
        }
        self.spec.flags = ACTIVITY_ENABLED;
        self.spec.priority = PRIORITY_MEDIUM;
        self.spec.timeout_ms = 0;
        if percent == 100 {
            self.spec.priority = PRIORITY_HIGH;
            self.spec.timeout_ms = 4000;
        } else {
            self.spec.flags |= ACTIVITY_KEEP_VISIBLE;
        }
        self.activity.update(self.spec)
    }
}

fn surface_spec(key: &str, kind: u32, width: f32, height: f32) -> SurfaceSpecV2 {
    let mut spec = SurfaceSpecV2 {
        kind,
        flags: SURFACE_ENABLED,
        width,
        height,
        ..Default::default()
    };
    spec.key[..key.len()].copy_from_slice(key.as_bytes());
    spec.title[..key.len()].copy_from_slice(key.as_bytes());
    spec
}

fn main() {
    let _ = Download::new as fn(&Host) -> Result<Download, Error>;
    let _ = Download::on_progress as fn(&mut Download, u8) -> Result<(), Error>;
}
