# quark-gui

Shared GUI library for Quark Downloader. Frontends bind widgets to the
reducer in this crate. They must not invent format lists or session JSON.

```
quark-gui              library: catalog, reduce, --script, C ABI
quark-gui-dispatch     binary: quark-downloader-gui (Linux Qt frontend compiled in)
quark-gui-win32        Windows frontend (in-process + --script)
quark-gui-qt           Qt 6 frontend (uses the system CuteCosmic theme on COSMIC)
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

`quark-downloader-gui` uses the frontend for this OS:

1. Windows — in-process Win32
2. macOS — AppKit helper beside the dispatcher, then `PATH`
3. Linux — Qt, compiled into `quark-downloader-gui` and run as
   `quark-downloader-gui --frontend qt` in its own process

`QUARK_GUI_FRONTEND` (id or full path) overrides helper discovery. On COSMIC,
Qt consumes the system CuteCosmic platform theme when it is installed. Only
`quark-downloader` and `quark-downloader-gui` are Linux executables.
