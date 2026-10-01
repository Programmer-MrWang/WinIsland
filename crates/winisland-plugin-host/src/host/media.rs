use super::PluginHost;

#[derive(Clone)]
pub struct MediaSnapshot {
    pub resource_id: u64,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub is_playing: bool,
    pub duration_ms: u64,
    pub position_ms: u64,
    pub available_controls: u32,
    pub cover: Vec<u8>,
}

impl PluginHost {
    pub fn current_media_id(&self) -> Option<u64> {
        let state = self.runtime.state.lock().ok()?;
        state
            .media
            .iter()
            .filter(|(id, _)| state.selected_media.is_none_or(|selected| selected == **id))
            .max_by_key(|(_, media)| media.sequence)
            .map(|(id, _)| *id)
    }

    pub fn select_media(&self, id: Option<u64>) -> winisland_plugin_api::PluginStatus {
        let Ok(mut state) = self.runtime.state.lock() else {
            return winisland_plugin_api::PluginStatus::Internal;
        };
        if id.is_some_and(|id| id != 0 && !state.media.contains_key(&id)) {
            return winisland_plugin_api::PluginStatus::StaleHandle;
        }
        state.selected_media = id;
        state.media_revision = state.media_revision.wrapping_add(1);
        drop(state);
        self.runtime.extensions.changed();
        winisland_plugin_api::PluginStatus::Ok
    }

    pub fn media_sources(&self) -> Vec<MediaSnapshot> {
        let Ok(state) = self.runtime.state.lock() else {
            return Vec::new();
        };
        let mut sources: Vec<_> = state
            .media
            .iter()
            .map(|(id, record)| MediaSnapshot {
                resource_id: *id,
                title: record.title.clone(),
                artist: record.artist.clone(),
                album: record.album.clone(),
                is_playing: record.flags
                    & winisland_plugin_api::types::v2::context::MEDIA_FLAG_PLAYING
                    != 0,
                duration_ms: record.duration_ms,
                position_ms: record.position_ms,
                available_controls: record.available_controls,
                cover: Vec::new(),
            })
            .collect();
        sources.sort_by_key(|source| source.resource_id);
        sources
    }

    pub fn dispatch_media_command(
        &self,
        resource_id: u64,
        command: u32,
        position_ms: u64,
    ) -> Result<(), crate::PluginHostError> {
        self.dispatch_media_request(resource_id, command, position_ms, None)
    }

    pub fn dispatch_media_request(
        &self,
        resource_id: u64,
        command: u32,
        position_ms: u64,
        completion: Option<(winisland_plugin_api::PluginToken, u64)>,
    ) -> Result<(), crate::PluginHostError> {
        use crate::resources::ResourceKind;
        let token = self
            .runtime
            .resources
            .owner(ResourceKind::Media, resource_id)
            .map_err(|_| crate::PluginHostError::Invalid("media resource is stale".into()))?;
        let entries = self.entries.borrow();
        let instance = entries
            .iter()
            .find(|entry| entry.token() == token)
            .ok_or_else(|| crate::PluginHostError::Invalid("media owner is unavailable".into()))?;
        instance.queue_media_command(resource_id, command, position_ms, completion)
    }

    pub fn media_snapshot(&self) -> Option<(u64, Option<MediaSnapshot>)> {
        let state = self.runtime.state.lock().ok()?;
        let source = state
            .media
            .iter()
            .filter(|(id, _)| state.selected_media.is_none_or(|selected| selected == **id))
            .max_by_key(|(_, media)| media.sequence);
        Some((
            state.media_revision,
            source.map(|(id, media)| MediaSnapshot {
                resource_id: *id,
                title: media.title.clone(),
                artist: media.artist.clone(),
                album: media.album.clone(),
                is_playing: media.flags
                    & winisland_plugin_api::types::v2::context::MEDIA_FLAG_PLAYING
                    != 0,
                duration_ms: media.duration_ms,
                position_ms: media.position_ms,
                available_controls: media.available_controls,
                cover: media.cover.clone(),
            }),
        ))
    }
}
