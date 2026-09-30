# Plugin API reference

The ABI v2 host exposes eleven service tables. Each page below documents one table: its capability, methods, data contract, lifetime, and current limits. Start with [ABI and lifecycle](/plugin-dev/abi-lifecycle) if you have not yet created a plugin descriptor.

Every service call takes the table's `prefix.context` and the host-issued `PluginToken`. It returns `PluginStatus`; created resources belong to that token. Query a table with `PluginHostV2.query(context, IFACE_*, IFACE_VERSION_1)`, check its size and version, then check each function slot you use. The SDK `Host` wrapper performs these table checks for its convenience methods.

| API | What it provides |
|---|---|
| [Context API](/plugin-dev/api/context) | Priority-based activity text for the island |
| [Media API](/plugin-dev/api/media) | Now-playing sources, cover art, timeline, and controls |
| [I18n API](/plugin-dev/api/i18n) | Plugin translation bundles |
| [Host State API](/plugin-dev/api/host-state) | Media and theme snapshots and change notifications |
| [Widget API](/plugin-dev/api/widget) | Grid widgets and validated draw lists |
| [Lyrics Transform API](/plugin-dev/api/lyrics-transform) | Parsed lyric-line transformation |
| [Settings API](/plugin-dev/api/settings) | Declarative plugin settings page |
| [Text API](/plugin-dev/api/text) | Text metrics and host font families |
| [Image API](/plugin-dev/api/image) | Decoded, uploaded, and album-art image handles |
| [Store API](/plugin-dev/api/store) | Plugin-scoped persistent byte values |
| [Log API](/plugin-dev/api/log) | Plugin-tagged diagnostic messages |

## Shared ABI rules

`PluginDescriptorV2.capabilities` must include the matching `CAP_*` bit for every service except Log. A successful table query alone does not grant access. The host rejects stale, foreign, or wrong-kind handles. Borrowed slices and input structures must remain valid until the synchronous call returns; callback data must remain valid until callbacks can no longer run.

The status values are `Ok`, `InvalidArgument`, `StaleHandle`, `CapabilityMissing`, `LimitExceeded`, `UnsupportedVersion`, `IoError`, and `Internal`. For binary output methods, a null buffer with zero capacity can be used to obtain the required byte length; a nonempty result reports `LimitExceeded` until enough capacity is supplied. Consult the method page for exceptions and callback rules.

These pages describe the current ABI v2 implementation. The [host services overview](/plugin-dev/services) gives a shorter tour, and the [SDK source](https://github.com/WinIslandProject/WinIsland/tree/master/crates/winisland-plugin-api/src) defines the exact Rust layouts.
