<table width="100%">
  <tr>
    <td align="left" width="120">
      <img src="icons/icon.png" alt="Quark Downloader" width="100" />
    </td>
    <td align="right">
      <h1>Quark Downloader</h1>
      <p>
        <a href="shard.yml">
          <img alt="Version 0.5.0" src="https://img.shields.io/badge/version-0.5.0-purple" />
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

## Dependencies

| Dependency         | Windows                           | macOS             | Linux                                                 |
| ------------------ | --------------------------------- | ----------------- | ----------------------------------------------------- |
| **yt-dlp**         | PATH or auto-download to `tools/` | PATH via Homebrew | PATH                                                  |
| **ffmpeg**         | PATH or bundled                   | PATH via Homebrew | PATH                                                  |
| **GUI (optional)** | Win32                             | AppKit UI         | [Tk](https://www.tcl.tk/) / `wish` (`apt install tk`) |

**Note:** Distro/apt yt-dlp is often too old. Prefer `pipx install yt-dlp` and [Node or Deno](https://github.com/yt-dlp/yt-dlp/wiki/EJS). Quark warns on stale versions and passes EJS flags when a JS runtime is on PATH.

**Build:** [Crystal](https://crystal-lang.org/) | [just](https://github.com/casey/just) | Windows installer: [Inno Setup 7](https://jrsoftware.org/isdl.php) + `packaging/quark-downloader.iss` | macOS app/DMG: Xcode Command Line Tools (`swiftc`) + `just dmg`

## Binaries

| Program | Purpose |
|---------|---------|
| `quark-downloader` | Full CLI - interactive in a terminal, or scriptable with flags |
| `quark-downloader-gui` | Thin UI that collects options and runs the CLI as a subprocess |
| `quark-downloader-gui-helper` | macOS only: native AppKit windows for the GUI (built with `swiftc`) |

The GUI queues multiple URLs (Add/Remove list) and downloads them sequentially with combined progress ("URL 2 of 5"). Playlist URLs download every item into a folder named after the playlist (see `playlist_folders`), with per-item progress and a failure summary.

Package maintainers can ship the CLI alone (`quark-downloader` on PATH) and optionally a GUI package that installs `quark-downloader-gui`, `quark-downloader-gui.tcl` (same directory), [`packaging/quark-downloader-gui.desktop`](packaging/quark-downloader-gui.desktop), and depends on **Tk** / `wish` (Linux). On macOS the GUI prefers the native `quark-downloader-gui-helper` beside the binary and falls back to Tk when it is missing.

Windows shortcuts from the installer open the GUI; the CLI remains in the install folder as **Quark Downloader (CLI)**. Use **Check for updates** in settings to compare against the latest [GitHub release](https://github.com/Aspenini/quark-downloader/releases) and open the installer download when a newer version is published.

## Configuration

On first run, Quark creates `quark-downloader.conf` under the user config directory:

| Setting | Values |
|---------|--------|
| `download_dir` | Default output folder (`~` is supported) |
| `yt_dlp` | `auto`, `path`, or `bundled` |
| `ffmpeg` | `auto`, `path`, or `bundled` |
| `gui_download_mode` | `progress` for the GUI progress dialog, or `external_cli` to open the CLI window after Download |
| `download_logs` | `true` or `false`; applies to both CLI and GUI downloads |
| `gui_theme` | `light` or `dark`; applies to the macOS/Linux GUI (Windows uses its native light UI) |
| `strip_video_ids` | `true` (default) drops the trailing ` [VIDEOID]` from filenames |
| `sanitize_filenames` | `true` (default) makes filenames mostly ASCII-safe on all platforms (`｜` -> `-`, accents transliterated, Windows-invalid characters removed) |
| `filename_spaces` | `keep` (default), `underscore`, `dash`, or `remove` |
| `playlist_folders` | `true` (default) saves playlist downloads into a folder named after the playlist (sanitized with the same rules) |

The download-naming settings are grouped under **Download Naming** in the GUI settings. The GUI gear button opens all settings without editing the file by hand. Logs are rotated in the config directory under `logs/`. Existing config files are updated with missing default keys on load.

## Commands

```bash
just run          # crystal run CLI
just run-gui      # crystal run GUI
just build        # release -> build/ (both binaries; UPX on Windows; AppKit helper on macOS)
just dmg          # macOS: build "Quark Downloader.app" + DMG into dist/
just clean
crystal spec      # run focused tests
```

The DMG is ad-hoc signed: after downloading, right-click > Open the first time (or `xattr -dr com.apple.quarantine "Quark Downloader.app"`). Inside the app bundle, downloaded tools are stored under `~/.config/quark-downloader/tools/`.

Build scripts live under [`scripts/`](scripts/README.md), grouped by platform.

### CLI (non-interactive)

```bash
quark-downloader --url 'https://example.com/video' --type video --format mp4 --output-dir ~/Downloads --no-pause
quark-downloader --url 'https://a/1' --url 'https://a/2'   # bulk: repeat --url; failures don't stop the queue
quark-downloader --batch-file urls.txt                     # one URL per line, # comments ignored
quark-downloader --url 'https://www.youtube.com/playlist?list=...'  # playlist -> own folder
quark-downloader --print-default-output-dir
```

Run with no arguments for the interactive prompt flow.

## Env (Windows)

`QUARK_SKIP_YTDLP_UPDATE=1` | `QUARK_SKIP_FFMPEG_DOWNLOAD=1`
