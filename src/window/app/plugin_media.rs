use winisland_plugin_api::*;

use crate::core::smtc::sessions::SessionRequest;

use super::App;

impl App {
    pub(super) fn update_plugin_media(&mut self) {
        let Some(host) = &self.plugin_host else {
            return;
        };
        let ext = &host.runtime().extensions;
        let active = host.capability_tokens(CAP_MEDIA_SESSION);
        self.smtc.plugin_sessions().set_active(active.clone());
        if self
            .plugin_ui
            .media_owner
            .is_some_and(|token| !active.contains(&token))
        {
            let _ = host.select_media(None);
            self.plugin_ui.media_owner = None;
            self.plugin_ui.media_selection = None;
        }
        let (mut sessions, results) = self.smtc.plugin_sessions().snapshot(ext.time_seconds());
        for result in results {
            if self.plugin_ui.media_selection == Some((result.token, result.sequence)) {
                if result.status == PluginStatus::Ok
                    && let Some(session) = result.selection
                {
                    let _ = host.select_media(if session == 0 { None } else { Some(0) });
                    self.plugin_ui.media_owner = (session != 0).then_some(result.token);
                }
                self.plugin_ui.media_selection = None;
            }
            ext.complete(result.token, result.sequence, result.status);
        }
        let current_plugin = host.current_media_id();
        if current_plugin.is_some() {
            for session in &mut sessions {
                session.flags &= !SESSION_CURRENT;
            }
        }
        for source in host.media_sources() {
            sessions.push(MediaSessionV2 {
                id: source.resource_id,
                flags: SESSION_PLUGIN
                    | (u32::from(source.is_playing) * SESSION_PLAYING)
                    | (u32::from(current_plugin == Some(source.resource_id)) * SESSION_CURRENT),
                available_controls: source.available_controls,
                duration_ms: source.duration_ms,
                position_ms: source.position_ms,
                sampled_at_seconds: ext.time_seconds(),
                title: str_to_fixed(&source.title),
                artist: str_to_fixed(&source.artist),
                album: str_to_fixed(&source.album),
                source: str_to_fixed("WinIsland plugin"),
                ..Default::default()
            });
        }
        ext.set_sessions(sessions);
    }

    pub(super) fn request_plugin_media(
        &mut self,
        token: PluginToken,
        sequence: u64,
        session: u64,
        command: u32,
        position_ms: u64,
    ) {
        let Some(host) = self.plugin_host.clone() else {
            return;
        };
        let sources = host.media_sources();
        let current = host.current_media_id();
        if command == SESSION_SELECT {
            self.plugin_ui.media_selection = Some((token, sequence));
        }
        let resolved = if session == 0 && command != SESSION_SELECT {
            current.unwrap_or(0)
        } else {
            session
        };
        if let Some(source) = sources.iter().find(|source| source.resource_id == resolved) {
            let status = if command == SESSION_SELECT {
                host.select_media(Some(resolved))
            } else {
                let command = match command {
                    SESSION_PLAY if source.is_playing => 0,
                    SESSION_PAUSE if !source.is_playing => 0,
                    SESSION_PLAY | SESSION_PAUSE => 1,
                    other => other,
                };
                if command == 0 {
                    PluginStatus::Ok
                } else {
                    match host.dispatch_media_request(
                        resolved,
                        command,
                        position_ms,
                        Some((token, sequence)),
                    ) {
                        Ok(()) => return,
                        Err(_) => PluginStatus::StaleHandle,
                    }
                }
            };
            if command == SESSION_SELECT && status == PluginStatus::Ok {
                self.plugin_ui.media_owner = Some(token);
                self.plugin_ui.media_selection = None;
            }
            host.runtime().extensions.complete(token, sequence, status);
        } else if resolved >= (1u64 << 32) {
            host.runtime()
                .extensions
                .complete(token, sequence, PluginStatus::StaleHandle);
        } else {
            let status = self.smtc.request_session(SessionRequest {
                token,
                sequence,
                session: resolved,
                command,
                position_ms,
            });
            if status != PluginStatus::Ok {
                host.runtime().extensions.complete(token, sequence, status);
            }
        }
    }
}
