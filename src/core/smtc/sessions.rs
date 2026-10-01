use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use winisland_platform::{MediaCommand, MediaContext, MediaSessionHandle};
use winisland_plugin_api::*;

#[derive(Clone, Default)]
pub struct SessionBridge(Arc<Mutex<BridgeState>>);

#[derive(Default)]
struct BridgeState {
    ready: bool,
    active: Vec<PluginToken>,
    snapshots: Vec<MediaSessionV2>,
    sampled_at: Option<Instant>,
    requests: VecDeque<SessionRequest>,
    results: Vec<SessionResult>,
}

pub struct SessionRequest {
    pub token: PluginToken,
    pub sequence: u64,
    pub session: u64,
    pub command: u32,
    pub position_ms: u64,
}

pub struct SessionResult {
    pub token: PluginToken,
    pub sequence: u64,
    pub status: PluginStatus,
    pub selection: Option<u64>,
}

impl SessionBridge {
    pub(super) fn set_ready(&self) {
        if let Ok(mut state) = self.0.lock() {
            state.ready = true;
        }
    }
    pub fn set_active(&self, tokens: Vec<PluginToken>) {
        if let Ok(mut state) = self.0.lock() {
            state
                .requests
                .retain(|request| tokens.contains(&request.token));
            state
                .results
                .retain(|result| tokens.contains(&result.token));
            state.active = tokens;
        }
    }

    pub fn request(&self, request: SessionRequest) -> PluginStatus {
        let Ok(mut state) = self.0.lock() else {
            return PluginStatus::Internal;
        };
        if !state.ready {
            return PluginStatus::IoError;
        }
        if state.requests.len() >= 128 {
            return PluginStatus::LimitExceeded;
        }
        state.requests.push_back(request);
        PluginStatus::Ok
    }

    pub fn snapshot(&self, now_seconds: f64) -> (Vec<MediaSessionV2>, Vec<SessionResult>) {
        let Ok(mut state) = self.0.lock() else {
            return (Vec::new(), Vec::new());
        };
        let mut sessions = state.snapshots.clone();
        let time = now_seconds
            - state
                .sampled_at
                .map_or(0.0, |at| at.elapsed().as_secs_f64());
        for session in &mut sessions {
            session.sampled_at_seconds = time.max(0.0);
        }
        (sessions, std::mem::take(&mut state.results))
    }
}

pub(super) struct SessionWorker {
    bridge: SessionBridge,
    ids: HashMap<u64, u64>,
    next: u64,
    selected: Option<u64>,
    selected_by: Option<PluginToken>,
}

impl SessionWorker {
    pub(super) fn new(bridge: SessionBridge) -> Self {
        Self {
            bridge,
            ids: HashMap::new(),
            next: 1,
            selected: None,
            selected_by: None,
        }
    }

    pub(super) fn refresh(
        &mut self,
        manager: &dyn MediaContext,
        allowed: &[String],
        enabled: bool,
    ) -> Option<Arc<dyn MediaSessionHandle>> {
        let active = self
            .bridge
            .0
            .lock()
            .map(|state| state.active.clone())
            .unwrap_or_default();
        if self
            .selected_by
            .is_some_and(|owner| !active.contains(&owner))
        {
            self.selected = None;
            self.selected_by = None;
        }
        if active.is_empty() {
            if let Ok(mut state) = self.bridge.0.lock() {
                state.snapshots.clear();
            }
            self.ids.clear();
            return None;
        }
        let sessions = if enabled {
            manager.sessions()
        } else {
            Vec::new()
        };
        self.ids.retain(|identity, _| {
            sessions
                .iter()
                .any(|session| session.identity() == *identity)
        });
        let mut handles = HashMap::new();
        for session in sessions {
            let identity = session.identity();
            if identity == 0 {
                continue;
            }
            let id = *self.ids.entry(identity).or_insert_with(|| {
                let id = self.next;
                self.next += 1;
                id
            });
            handles.insert(id, session);
        }
        if self.selected.is_some_and(|id| !handles.contains_key(&id)) {
            self.selected = None;
        }
        let automatic = enabled
            .then(|| super::session::get_target_session(manager, allowed))
            .flatten();
        let automatic_id = automatic
            .as_ref()
            .and_then(|session| self.ids.get(&session.identity()).copied());
        let requests: Vec<_> = self
            .bridge
            .0
            .lock()
            .map(|mut state| state.requests.drain(..).collect())
            .unwrap_or_default();
        let mut results = Vec::new();
        for request in requests {
            if !active.contains(&request.token) {
                continue;
            }
            let id = if request.session == 0 {
                self.selected.or(automatic_id)
            } else {
                Some(request.session)
            };
            let status = if request.command == SESSION_SELECT && request.session == 0 {
                self.selected = None;
                self.selected_by = None;
                PluginStatus::Ok
            } else if let Some((id, session)) = id.and_then(|id| handles.get(&id).map(|s| (id, s)))
            {
                if request.command == SESSION_SELECT {
                    self.selected = Some(id);
                    self.selected_by = Some(request.token);
                    PluginStatus::Ok
                } else {
                    let command = match request.command {
                        1 => Some(MediaCommand::Toggle),
                        2 => Some(MediaCommand::Previous),
                        3 => Some(MediaCommand::Next),
                        4 => Some(MediaCommand::Seek(Duration::from_millis(
                            request.position_ms,
                        ))),
                        SESSION_PLAY => Some(MediaCommand::Play),
                        SESSION_PAUSE => Some(MediaCommand::Pause),
                        _ => None,
                    };
                    match command.map(|command| session.send(command)) {
                        Some(Ok(true)) => PluginStatus::Ok,
                        Some(Ok(false)) => PluginStatus::InvalidArgument,
                        Some(Err(_)) => PluginStatus::IoError,
                        None => PluginStatus::InvalidArgument,
                    }
                }
            } else {
                PluginStatus::StaleHandle
            };
            results.push(SessionResult {
                token: request.token,
                sequence: request.sequence,
                status,
                selection: (request.command == SESSION_SELECT).then_some(request.session),
            });
        }
        let current = self.selected.or(automatic_id);
        let mut snapshots: Vec<_> = handles
            .iter()
            .map(|(id, session)| {
                let track = session.track().unwrap_or_default();
                let timeline = session.timeline().unwrap_or_default();
                let controls = session.controls();
                MediaSessionV2 {
                    id: *id,
                    flags: (u32::from(session.is_playing()) * SESSION_PLAYING)
                        | (u32::from(current == Some(*id)) * SESSION_CURRENT),
                    available_controls: u32::from(controls.play || controls.pause)
                        | (u32::from(controls.previous) << 1)
                        | (u32::from(controls.next) << 2)
                        | (u32::from(controls.seek) << 3),
                    duration_ms: timeline.duration.as_millis() as u64,
                    position_ms: timeline.position.as_millis() as u64,
                    source: str_to_fixed(&session.source_app_id().unwrap_or_default()),
                    title: str_to_fixed(&track.title),
                    artist: str_to_fixed(&track.artist),
                    album: str_to_fixed(&track.album),
                    ..Default::default()
                }
            })
            .collect();
        snapshots.sort_by_key(|session| session.id);
        if let Ok(mut state) = self.bridge.0.lock() {
            state.snapshots = snapshots;
            state.sampled_at = Some(Instant::now());
            state.results.extend(results);
        }
        crate::platform::wake();
        self.selected.and_then(|id| handles.remove(&id))
    }
}
