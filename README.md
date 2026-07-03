<table width="100%">
  <tr>
    <td align="left" width="120">
      <img src="icons/icon.png" alt="Quark Downloader" width="100" />
    </td>
    <td align="right">
      <h1>Quark Downloader</h1>
      <p>
        <a href="https://github.com/Aspenini/quark-downloader/actions/workflows/rust.yml">
          <img alt="Rust CI" src="https://img.shields.io/github/actions/workflow/status/Aspenini/quark-downloader/rust.yml?branch=main&amp;label=Rust%20CI&amp;color=orange" />
        </a>
        <a href="https://github.com/Aspenini/quark-downloader/releases">
          <img alt="GitHub release" src="https://img.shields.io/github/v/release/Aspenini/quark-downloader?label=release" />
        </a>
        <a href="https://aur.archlinux.org/packages/quark-downloader">
          <img alt="AUR version" src="https://img.shields.io/aur/version/quark-downloader?label=AUR&amp;logo=archlinux&amp;cacheSeconds=3600" />
        </a>
      </p>
    </td>
  </tr>
</table>

A friendly [yt-dlp](https://github.com/yt-dlp/yt-dlp) wrapper — a scriptable CLI and a simple GUI
for downloading audio/video and whole playlists, with automatic filename cleanup. Written in Rust.

## Dependencies

| Dependency | Windows | macOS | Linux |
| ---------- | ------- | ----- | ----- |
| **yt-dlp** | PATH, or auto-download to `tools/` | PATH (`brew install yt-dlp`) | PATH |
| **ffmpeg** (only for format conversion) | PATH, or auto-download | PATH (`brew install ffmpeg`) | PATH |
| **JS runtime** (YouTube) | any of deno / node / quickjs / bun on PATH | same | same |

**Note:** Distro/apt yt-dlp is often too old for YouTube. Prefer `pipx install yt-dlp` plus a
[JS runtime](https://github.com/yt-dlp/yt-dlp/wiki/EJS). Quark warns on stale versions and passes
the EJS flags automatically when a runtime is on PATH.

## Workspace

A Cargo workspace under `crates/`:

| Crate | What it is |
| ----- | ---------- |
| `quark-core` | The engine: config, tool management, and yt-dlp orchestration. No UI. |
| `quark-gui` | **QuarkGUI** — a standalone, reusable cross-platform UI toolkit. Write one UI; render it through any backend. Not tied to this app. |
| `quark-cli` | The `quark-downloader` binary (terminal). |
| `quark-downloader-gui` | The GUI binary; builds its windows with QuarkGUI and drives `quark-core` in-process. |

### GUI backends

QuarkGUI renders through selectable native backends. The normal build includes every backend
supported by its target. Choose one in the GUI settings; the saved backend is used the next time
the app starts.

| Setting | Platform | Toolkit (`cargo` feature) |
| ------- | -------- | ------------------------- |
| `win32` | Windows; default | native Win32 form, progress, and dialogs (`native-windows`) |
| `cocoa` | macOS; default | native AppKit form, progress, and dialogs (`native-cocoa`) |
| `gtk` | macOS and Linux; Linux default | GTK 4 via gtk4-rs (`native-gtk`, needs GTK 4) |
| `kirigami` | Windows, macOS, and Linux | Qt Widgets via a cxx bridge — Breeze/Kirigami look on KDE (`native-kirigami`, needs Qt 6) |

All four are complete implementations of the same renderer interface. Requesting a backend that
isn't compiled into the current build falls back to that platform's default native backend.
`winui` is accepted as a legacy alias for `win32`.

## Build & run

Needs a [Rust toolchain](https://rustup.rs/). [just](https://github.com/casey/just) recipes wrap
the common cargo commands.

```bash
just build              # release build with this platform's GUI backends
just run -- --help      # run the CLI
just run-gui            # run the GUI with all backends for this platform
just demo-gui           # standalone QuarkGUI example (proves the toolkit is reusable)
just test               # test the workspace with this platform's GUI backends
just lint               # clippy, warnings as errors
```

### Test Windows from macOS or Linux

The Windows GNU target can be cross-compiled and exercised through Wine without a Windows machine.
Install the target plus MinGW-w64 and Wine:

```bash
rustup target add x86_64-pc-windows-gnu

# macOS
brew install mingw-w64
brew install --cask wine-stable

# Debian/Ubuntu
sudo apt install mingw-w64 wine
```

Then use the Windows recipes:

```bash
just windows-build                 # cross-compile the Windows CLI and GUI
just windows-test                  # run Windows test binaries through Wine
just windows-run-gui               # launch the .exe with the native Win32 backend
just windows-run-cli --help        # run the Windows CLI
```

Wine state is isolated under `target/wine`. Set `WINE=/path/to/wine` if the executable is not
named `wine`. The cross-compiled build contains Win32, which is also the Windows default.

The Linux build links real GTK and Qt system libraries; install `libgtk-4-dev` and `qt6-base-dev`
on Debian-like systems. On macOS, install `gtk4` and `qt` with Homebrew to build every supported
backend. Windows needs Qt 6 with `qmake` on `PATH` for the optional Kirigami backend; the native
Win32-only Wine cross-test does not require Qt. A scripted, self-closing progress demo
smoke-tests any backend without interaction, e.g.
`QUARK_GUI_BACKEND=gtk cargo run -p quark-gui --example progress --no-default-features --features native-gtk`.

After `just build`, launch the release binaries directly:

```bash
./target/release/quark-downloader       # terminal interface
./target/release/quark-downloader-gui   # graphical interface
```

On Windows, use the corresponding `.exe` files. Installed Linux packages also provide separate
“Quark Downloader” (GUI) and “Quark Downloader (CLI)” application-menu entries.

## CLI

```bash
quark-downloader --url 'https://example.com/video' --type video --format mp4 --output-dir ~/Downloads
quark-downloader --url 'https://a/1' --url 'https://a/2'   # bulk: repeat --url; failures don't stop the queue
quark-downloader --batch-file urls.txt                     # one URL per line, # comments ignored
quark-downloader --url 'https://www.youtube.com/playlist?list=...'  # playlist -> its own folder
quark-downloader --print-default-output-dir
```

Run with no arguments for the interactive prompt flow. A live progress bar shows in a terminal;
output falls back to plain lines when piped or logged.

## Configuration

On first run, Quark writes `quark-downloader.toml` in the user config directory
(`%APPDATA%\quark-downloader` on Windows, `$XDG_CONFIG_HOME/quark-downloader` elsewhere). A legacy
`quark-downloader.conf` from an older build is migrated to TOML automatically (the original is kept
as `.conf.bak`).

| Setting | Values |
| ------- | ------ |
| `download_dir` | Default output folder (`~` supported) |
| `yt_dlp` / `ffmpeg` | `auto`, `path`, or `bundled` |
| `gui_backend` | `win32`, `cocoa`, `gtk`, `kirigami`; default depends on the platform |
| `gui_download_mode` | `progress` (in-app progress window) or `external_cli` (open the CLI in a terminal) |
| `gui_theme` | `light` or `dark` |
| `download_logs` | `true` / `false` — rotated logs under the config dir's `logs/` |
| `strip_video_ids` | `true` drops the trailing ` [VIDEOID]` from filenames |
| `sanitize_filenames` | `true` makes filenames mostly ASCII-safe on all platforms |
| `filename_spaces` | `keep`, `underscore`, `dash`, or `remove` |
| `playlist_folders` | `true` saves a playlist into a folder named after it |

The GUI's gear/**Settings** button edits all of these without touching the file by hand.

## Using QuarkGUI on its own

QuarkGUI has no dependency on the downloader and can be pulled into any project:

```rust
use quark_gui::{App, Backend};
use quark_gui::model::{FormSpec, WindowSpec, Theme, Field, FormOutcome};

let app = App::new(Backend::Auto);
let mut form = FormSpec::new(WindowSpec::new("Demo", Theme::Light));
form.fields.push(Field::Text { id: "name".into(), label: "Name".into(), value: String::new() });
if let FormOutcome::Submit(values) = app.run_form(form) {
    println!("{}", values.text("name"));
}
```

See [`crates/quark-gui/examples/standalone.rs`](crates/quark-gui/examples/standalone.rs).

## License

MIT — see [LICENSE](LICENSE).
