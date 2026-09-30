# Lyrics Transform API

`LyricsTransformApiV2` registers a callback that changes parsed lyric text before display. Declare `CAP_LYRICS` and query `IFACE_LYRICS_TRANSFORM`. The SDK `host.lyrics()?.register(|line| ...)` manages callback storage for a text-to-text closure.

## Methods

Both methods take `context, token` first and return `PluginStatus`.

| Method | Remaining parameters | Result |
|---|---|---|
| `register` | `*const LyricsTransformerDataV2`, `*mut ResourceId` | Registers a transformer and writes its ID. |
| `release` | `ResourceId` | Removes it when no callback is in flight. |

## Transformation callback

`LyricsTransformerDataV2` requires `on_transform`; its reserved `flags` must be zero. The callback receives a `LyricsTextV2` containing the line timestamp, flags, and borrowed UTF-8 text. It is called twice for each line: first with null output and zero capacity to ask for the required byte length, then with a writable buffer. Write `out_len` in both passes and return `PluginStatus::Ok` on success. Keep the output stable between passes.

When `LYRICS_TEXT_FLAG_WORD_SYNCED` is set, preserve the same Unicode character count to keep word timing boundaries. The input text is valid only during the callback. Keep `callback_data` valid until release succeeds; releasing during an active callback returns `LimitExceeded`. The current quota is four transformers per plugin.

See the [lyrics example](https://github.com/WinIslandProject/WinIsland/blob/master/crates/winisland-plugin-api/examples/lyrics_transform.rs) and [callback type](https://github.com/WinIslandProject/WinIsland/blob/master/crates/winisland-plugin-api/src/types/v2/lyrics.rs).

[All plugin APIs](/plugin-dev/api)
