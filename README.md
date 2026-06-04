<table width="100%">
  <tr>
    <td align="left" width="120">
      <img src="icons/icon.png" alt="Quark Downloader" width="100" />
    </td>
    <td align="right">
      <h1>Quark Downloader</h1>
      <p>
        <a href="https://github.com/Aspenini/quark-downloader/releases">
          <img alt="GitHub release" src="https://img.shields.io/github/v/release/Aspenini/quark-downloader?label=release" />
        </a>
        <a href="shard.yml">
          <img alt="Version 0.3.0" src="https://img.shields.io/badge/version-0.3.0-black" />
        </a>
        <a href="https://aur.archlinux.org/packages/quark-downloader">
          <img alt="AUR version" src="https://img.shields.io/aur/version/quark-downloader?label=AUR&amp;logo=archlinux&amp;cacheSeconds=3600" />
        </a>
      </p>
    </td>
  </tr>
</table>

## Dependencies

| | Windows | macOS / Linux |
|---|---------|----------------|
| **yt-dlp** | PATH or auto-download to `tools/` | PATH - keep it current for YouTube |
| **ffmpeg** | PATH, `bundled-tools/` -> `build/tools/` on build, or auto-download | PATH (`brew install ffmpeg`, etc.) |
| **GUI (optional)** | Built-in Win32 dialog | [Tk](https://www.tcl.tk/) / `wish` (`apt install tk`, `brew install tcl-tk`) |

**Note:** Distro/apt yt-dlp is often too old. Prefer `pipx install yt-dlp` and [Node or Deno](https://github.com/yt-dlp/yt-dlp/wiki/EJS). Quark warns on stale versions and passes EJS flags when a JS runtime is on PATH.

**Build:** [Crystal](https://crystal-lang.org/) | [just](https://github.com/casey/just) | Windows installer: [Inno Setup 7](https://jrsoftware.org/isdl.php) + `packaging/quark-downloader.iss`

## Binaries

| Program | Purpose |
|---------|---------|
| `quark-downloader` | Full CLI - interactive in a terminal, or scriptable with flags |
| `quark-downloader-gui` | Thin UI that collects options and runs the CLI as a subprocess |

Package maintainers can ship the CLI alone (`quark-downloader` on PATH) and optionally a GUI package that installs `quark-downloader-gui`, `quark-downloader-gui.tcl` (same directory), [`packaging/quark-downloader-gui.desktop`](packaging/quark-downloader-gui.desktop), and depends on **Tk** / `wish` (Linux/macOS).

Windows shortcuts from the installer open the GUI; the CLI remains in the install folder as **Quark Downloader (CLI)**.

## Configuration

On first run, Quark creates `quark-downloader.conf` under the user config directory:

| Setting | Values |
|---------|--------|
| `download_dir` | Default output folder (`~` is supported) |
| `yt_dlp` | `auto`, `path`, or `bundled` |
| `ffmpeg` | `auto`, `path`, or `bundled` |
| `gui_download_mode` | `progress` for the GUI progress dialog, or `external_cli` to open the CLI window after Download |
| `download_logs` | `true` or `false`; applies to both CLI and GUI downloads |

The GUI gear button opens these settings without editing the file by hand. Logs are rotated in the config directory under `logs/`. Existing config files are updated with missing default keys on load.

## Commands

```bash
just run          # crystal run CLI
just run-gui      # crystal run GUI
just build        # release -> build/ (both binaries; UPX on Windows)
just clean
crystal spec      # run focused tests
```

Build scripts live under [`scripts/`](scripts/README.md), grouped by platform.

### CLI (non-interactive)

```bash
quark-downloader --url 'https://example.com/video' --type video --format mp4 --output-dir ~/Downloads --no-pause
quark-downloader --print-default-output-dir
```

Run with no arguments for the interactive prompt flow.

## Env (Windows)

`QUARK_SKIP_YTDLP_UPDATE=1` | `QUARK_SKIP_FFMPEG_DOWNLOAD=1`
