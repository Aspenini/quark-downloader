# quark-gui

Shared GUI library for Quark Downloader. Frontends bind widgets to the
reducer in this crate. They must not invent format lists or session JSON.

```
quark-gui              library: catalog, reduce, --script, C ABI
quark-gui-dispatch     binary: quark-downloader-gui (Linux frontends compiled in)
quark-gui-win32        Windows frontend (in-process + --script)
quark-gui-cosmic       COSMIC / iced (linked into the GUI)
quark-gui-kirigami     Kirigami (linked into the GUI when Qt 6 is present; qml/ next to the GUI)
quark-gui-appkit       --script runner; visual UI is still Swift AppKit
```

## Contract

`cargo test -p quark-gui --test contract` is the spec. Every frontend
`--script` binary is checked against the same fixtures by
`--test frontends` when that binary exists on the host.

Settings are included in session JSON only after Save. Empty queue and
empty output use the strings in `copy.rs`. Download flushes the URL field
first. Switching audio/video resets format to `original`. Theme `system`
follows the desktop (Plasma, COSMIC light/dark, or macOS appearance).

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

`quark-downloader-gui` picks a frontend and speaks:

1. `QUARK_GUI_FRONTEND` — id or full path
2. Config `gui_frontend` (`auto`, `cosmic`, `kirigami`, `win32`, `appkit`)
3. macOS AppKit helper beside the dispatcher, then `PATH`

Windows `auto`/`win32` is in-process Win32. Linux `auto` prefers Kirigami on
KDE and COSMIC otherwise. COSMIC and Kirigami are compiled into
`quark-downloader-gui` and run as `quark-downloader-gui --frontend <id>`
so each toolkit gets its own process. Only `quark-downloader` and
`quark-downloader-gui` are Linux executables.
