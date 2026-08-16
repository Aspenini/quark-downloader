# quark-gui

Shared GUI library for Quark Downloader. Frontends bind widgets to the
reducer in this crate. They must not invent format lists or session JSON.

```
quark-gui              library: catalog, reduce, --script, C ABI
quark-gui-dispatch     binary: quark-downloader-gui
quark-gui-win32        Windows frontend (in-process + --script)
quark-gui-gtk          Linux helper: --session / --progress / --message / --script
quark-gui-appkit       --script runner; visual UI is still Swift AppKit
```

## Contract

`cargo test -p quark-gui --test contract` is the spec. Every frontend
`--script` binary is checked against the same fixtures by
`--test frontends` when that binary exists on the host.

Settings are included in session JSON only after Save. Empty queue and
empty output use the strings in `copy.rs`. Download flushes the URL field
first. Switching audio/video resets format to `original`.

## `--script`

Read one JSON document on stdin, print one session JSON object on stdout.
Does not open a window or need a display.

```json
{
  "args": { "default_dir": "/tmp/dl" },
  "events": [
    { "add_url": "https://example.com/a" },
    { "download": true }
  ]
}
```

## Dispatcher protocol

`quark-downloader-gui` discovers a helper and speaks:

1. `QUARK_GUI_FRONTEND` — id or full path
2. Config `gui_frontend` (`auto` or `gtk` on Linux)
3. Sibling of the dispatcher, then `PATH`

Windows uses in-process Win32. macOS visual UI is `quark-downloader-gui-appkit`
(Swift). `--session` still uses positional argv; helpers print JSON `v:1`.
Legacy `__SESSION__` lines are still parsed.
