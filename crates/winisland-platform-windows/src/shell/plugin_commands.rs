use std::collections::{HashMap, VecDeque};
use std::sync::{LazyLock, Mutex};

use windows::Win32::UI::Input::KeyboardAndMouse::{
    MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, MOD_WIN, RegisterHotKey, UnregisterHotKey,
};
use winisland_platform::{Hotkey, PlatformError, PluginCommand};

struct Binding {
    native: i32,
    hotkey: Hotkey,
}

struct State {
    next: i32,
    bindings: HashMap<u64, Binding>,
    presses: VecDeque<u64>,
}

static STATE: LazyLock<Mutex<State>> = LazyLock::new(|| {
    Mutex::new(State {
        next: 0x4000,
        bindings: HashMap::new(),
        presses: VecDeque::new(),
    })
});

pub(super) fn update(commands: &[PluginCommand]) -> Vec<(u64, PlatformError)> {
    let Ok(mut state) = STATE.lock() else {
        return Vec::new();
    };
    state.bindings.retain(|id, binding| {
        let keep = commands.iter().any(|command| {
            command.id == *id && command.enabled && command.hotkey == Some(binding.hotkey)
        });
        if !keep {
            // SAFETY: This is the UI thread that registered these thread-bound hotkeys.
            let _ = unsafe { UnregisterHotKey(None, binding.native) };
        }
        keep
    });
    let mut errors = Vec::new();
    for command in commands.iter().filter(|command| command.enabled) {
        let Some(key) = command.hotkey else {
            continue;
        };
        if state.bindings.contains_key(&command.id) {
            continue;
        }
        if !key.key.is_ascii_alphanumeric() || state.next > 0xbfff {
            errors.push((command.id, PlatformError::Unavailable("plugin hotkey")));
            continue;
        }
        let mut modifiers = MOD_NOREPEAT;
        if key.ctrl {
            modifiers |= MOD_CONTROL;
        }
        if key.alt {
            modifiers |= MOD_ALT;
        }
        if key.shift {
            modifiers |= MOD_SHIFT;
        }
        if key.win {
            modifiers |= MOD_WIN;
        }
        let native = state.next;
        state.next += 1;
        // SAFETY: A null HWND registers the hotkey with the current winit message-pumping thread.
        match unsafe {
            RegisterHotKey(
                None,
                native,
                modifiers,
                u32::from(key.key.to_ascii_uppercase()),
            )
        } {
            Ok(()) => {
                state.bindings.insert(
                    command.id,
                    Binding {
                        native,
                        hotkey: key,
                    },
                );
            }
            Err(error) => errors.push((command.id, PlatformError::backend(error))),
        }
    }
    errors
}

pub(crate) fn pressed(native: usize) {
    if let Ok(mut state) = STATE.lock()
        && let Some(id) = state
            .bindings
            .iter()
            .find_map(|(id, binding)| (binding.native as usize == native).then_some(*id))
    {
        if state.presses.len() < 128 {
            state.presses.push_back(id);
        }
        crate::window::wake();
    }
}

pub(super) fn poll() -> Vec<u64> {
    STATE
        .lock()
        .map(|mut state| state.presses.drain(..).collect())
        .unwrap_or_default()
}
