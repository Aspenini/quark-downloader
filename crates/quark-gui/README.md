# quark-gui

Shared GUI library for Quark Downloader. Frontends bind widgets to the
reducer in this crate. They must not invent format lists or session JSON.

```
quark-gui              library: catalog, reduce, --script, C ABI
quark-gui-dispatch     binary: quark-downloader-gui
quark-gui-win32        Windows frontend (in-process + --script)
quark-gui-gtk          GTK 4 helper (system libgtk-4)
quark-gui-cosmic       COSMIC / iced helper (Linux UI; system Wayland/Vulkan)
quark-gui-kirigami     Kirigami helper (system Qt 6 + distro Kirigami QML)
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
2. Config `gui_frontend` (`auto`, `gtk`, `cosmic`, `kirigami`, `win32`, `appkit`)
3. Sibling of the dispatcher, then `PATH`

Windows `auto`/`win32` is in-process Win32. Other picks launch a helper.
macOS `auto` prefers AppKit. Linux `auto` tries gtk, then cosmic, then kirigami.
GTK and Kirigami dynamically link system `libgtk-4` / Qt 6. COSMIC is a
separate iced helper so it is not linked into the dispatcher.
