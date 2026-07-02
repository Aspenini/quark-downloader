# QuarkGUI

A small, reusable cross-platform UI toolkit: describe a form, progress window,
or message dialog once with plain Rust data types, then render it through the
backend of your choice — Slint by default, or the platform's native toolkit.

QuarkGUI grew out of [Quark Downloader](https://github.com/Aspenini/quark-downloader)
but has no dependency on it and can be used by any program.

## Design

Three view types cover the "small utility app" space:

- **Form** — labelled inputs (text, path + browse, dropdown, radio group,
  checkbox, editable list, section headings) plus submit/cancel/extra buttons.
- **Progress** — a bar, status/ETA/queue labels, and a cancel button, driven by
  a channel of updates from your worker thread.
- **Message** — a modal info/error dialog.

Every backend implements the same `Renderer` trait, so the calling code never
changes:

```rust
use quark_gui::{App, Backend};
use quark_gui::model::{Field, FormOutcome, FormSpec, Theme, WindowSpec};

let app = App::new(Backend::Auto);
let mut form = FormSpec::new(WindowSpec::new("Demo", Theme::Light));
form.fields.push(Field::Text { id: "name".into(), label: "Name".into(), value: String::new() });
if let FormOutcome::Submit(values) = app.run_form(form) {
    println!("hello, {}", values.text("name"));
}
```

## Backends

| `Backend` | Cargo feature | Notes |
| --------- | ------------- | ----- |
| `Slint` | `slint` (default) | Pure Rust, no system dependencies, all platforms |
| `Cocoa` | `native-cocoa` | Native AppKit (macOS) |
| `Win32` | `native-windows` | Native Win32 (Windows) |
| `Gtk` | `native-gtk` | GTK 4 via gtk4-rs — needs the GTK 4 libraries |
| `Kirigami` | `native-kirigami` | Qt Widgets via a cxx bridge — needs Qt 6 (`qmake` on PATH); Breeze/Kirigami look on KDE |
| `Headless` | — | No UI; accepts form defaults (tests, non-interactive fallback) |

Requesting a backend that isn't compiled in falls back to Slint automatically
(or Headless when Slint is disabled too), so shipping one binary with optional
native features is safe.

## Examples

```bash
# Interactive form demo (Slint):
cargo run -p quark-gui --example standalone

# Self-closing progress demo — also a no-interaction smoke test for any backend:
cargo run -p quark-gui --example progress
QUARK_GUI_BACKEND=gtk cargo run -p quark-gui --example progress \
    --no-default-features --features native-gtk
```

## License

MIT — see [LICENSE](LICENSE).
