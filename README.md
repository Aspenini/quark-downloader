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
        <a href="https://aur.archlinux.org/packages/quark-downloader">
          <img alt="AUR version" src="https://img.shields.io/aur/version/quark-downloader?label=AUR&amp;logo=archlinux&amp;cacheSeconds=3600" />
        </a>
      </p>
    </td>
  </tr>
</table>

## Dependencies

| Dependency         | Windows                           | macOS             | Linux                                                 | Android |
| ------------------ | --------------------------------- | ----------------- | ----------------------------------------------------- | ------- |
| **yt-dlp**         | PATH or auto-download to `tools/` | PATH via Homebrew | PATH (package manager / `pipx`)                       | bundled (youtubedl-android) |
| **ffmpeg**         | PATH or bundled                   | PATH via Homebrew | PATH (package manager)                                | bundled |
| **GUI (optional)** | Win32 | AppKit | Qt 6; CuteCosmic integration on COSMIC | Jetpack Compose |

**Note:** Distro/apt yt-dlp is often too old. Prefer `pipx install yt-dlp` and [Node or Deno](https://github.com/yt-dlp/yt-dlp/wiki/EJS). Quark warns on stale versions and passes EJS flags when a JS runtime is on PATH.

**Build:** [Rust](https://www.rust-lang.org/) 1.85+ (edition 2024); Linux GUI also needs Qt 6 Declarative development files | [just](https://github.com/casey/just) | Windows installer: [Inno Setup 7](https://jrsoftware.org/isdl.php) + `packaging/quark-downloader.iss` | macOS app/DMG: Xcode Command Line Tools + `just dmg` | Android: JDK 17 + NDK, `just android-release` (see [`android/README.md`](android/README.md))

## Binaries

| Program | Purpose |
|---------|---------|
| `quark-downloader` | Full CLI - interactive in a terminal, or scriptable with flags |
| `quark-downloader-gui` | Qt frontend on Linux; AppKit frontend on macOS; Win32 frontend on Windows |
| Android APK | Compose app; GitHub Releases only (not Play Store). GPL-3.0 because it links youtubedl-android. |

Android license text, third-party notices, and corresponding-source details are
in [`android/`](android/README.md). Desktop binaries and reusable shared crates
remain MIT-licensed.

The GUI queues multiple URLs (Add/Remove list) and downloads them sequentially with combined progress ("URL 2 of 5"). Playlist URLs download every item into a folder named after the playlist (see `playlist_folders`), with per-item progress and a failure summary.

Package maintainers can ship the CLI alone (`quark-downloader` on PATH) and optionally a GUI package that installs `quark-downloader-gui`, [`packaging/quark-downloader-gui.desktop`](packaging/quark-downloader-gui.desktop). Linux builds link the Qt 6 frontend when Qt Declarative is present. Qt automatically uses the installed [CuteCosmic](https://github.com/IgKh/cutecosmic) platform theme in a COSMIC session. macOS builds compile the AppKit frontend into `quark-downloader-gui`. All frontends share the `quark-gui` catalog, reducer, and `--script` contract.

Windows shortcuts from the installer open the GUI; the CLI remains in the install folder as **Quark Downloader (CLI)**. Use **Check for updates** in settings to compare against the latest [GitHub release](https://github.com/Aspenini/quark-downloader/releases) and open the installer download when a newer version is published.

## Configuration

On first run, Quark creates `quark-downloader.conf` under the user config directory:

| Setting | Values |
|---------|--------|
| `download_dir` | Default output folder (`~` is supported) |
| `yt_dlp` | **Windows only:** `auto`, `path`, or `bundled`. macOS/Linux always use PATH (Homebrew / package manager). |
| `ffmpeg` | **Windows only:** `auto`, `path`, or `bundled`. macOS/Linux always use PATH. |
| `gui_download_mode` | `progress` for the GUI progress dialog, or `external_cli` to open the CLI window after Download |
| `download_logs` | `true` or `false`; applies to both CLI and GUI downloads |
| `open_output_dir` | `true` or `false` (default `false`); GUI only, opens the output folder when a download finishes |
| `gui_theme` | `system` (default), `light`, or `dark`. `system` follows Qt/CuteCosmic on Linux, macOS appearance in AppKit, and native Windows colors. |
| `strip_video_ids` | `true` (default) drops the trailing ` [VIDEOID]` from filenames |
| `sanitize_filenames` | `true` (default) makes filenames mostly ASCII-safe on all platforms (`｜` -> `-`, accents transliterated, Windows-invalid characters removed) |
| `filename_spaces` | `keep` (default), `underscore`, `dash`, or `remove` |
| `playlist_folders` | `true` (default) saves playlist downloads into a folder named after the playlist (sanitized with the same rules) |

The download-naming settings are grouped under **Download Naming** in the GUI settings. The GUI gear button opens all settings without editing the file by hand. Logs are rotated in the config directory under `logs/`. Existing config files are updated with missing default keys on load.

## Commands

```bash
just run          # cargo run CLI
just run-gui      # cargo run GUI dispatcher
just build        # release -> build/ (CLI + GUI; UPX CLI)
just dmg          # macOS: build "Quark Downloader.app" + DMG into dist/
just windows-release # Windows: unsigned installer
just dmg-release     # macOS: ad-hoc signed, unnotarized DMG
just android-release # Android: signed APK using the release keystore
just test         # cargo test --workspace
just clean
```

The DMG is ad-hoc signed: after downloading, right-click > Open the first time (or `xattr -dr com.apple.quarantine "Quark Downloader.app"`). On macOS and Linux, install **yt-dlp** and **ffmpeg** yourself (`brew install yt-dlp ffmpeg`, or your distro / `pipx`); Quark does not bundle or auto-download them there. **Do not run with sudo** — that writes config and downloads into root's home.

**CLI color:** Interactive and TTY output uses ANSI colors when supported. Disable with `NO_COLOR=1` or force with `FORCE_COLOR=1`.

**Stall watchdog:** Playlist items that go silent too long are skipped (`QUARK_STALL_TIMEOUT_SEC`, default ~75s after output starts, ~90s grace before first output). Single-video stalls warn instead of killing.

## Release

Before tagging, run formatting, strict Clippy, all tests, a release build, and
confirm the GitHub Actions Rust, MSRV, Android, and audit jobs are green. Test a
real audio conversion, video, playlist, cancellation, and update check on each
supported platform.

Windows artifacts are intentionally unsigned and may trigger SmartScreen.
macOS releases are ad-hoc signed and unnotarized, so users may need to
right-click **Open** on first launch. Android releases use
`%USERPROFILE%\quark-release.jks` on Windows (or `$HOME/quark-release.jks` on
Unix) and prompt securely for the password. Preserve that keystore for every
future Android update.

Create a `vVERSION` tag, replace `sha256s=('SKIP')` in `packaging/PKGBUILD`
with the tagged source archive's SHA-256, run `makepkg --printsrcinfo` to update
the AUR `.SRCINFO`, and upload assets using the names expected by the in-app
updater.

### CLI (non-interactive)

```bash
quark-downloader --url 'https://example.com/video' --type video --format mp4 --output-dir ~/Downloads --no-pause
quark-downloader --url 'https://a/1' --url 'https://a/2'   # bulk: repeat --url; failures don't stop the queue
quark-downloader --batch-file urls.txt                     # one URL per line, # comments ignored
quark-downloader --url '…' --emit-result-json              # final __RESULT__ JSON line for tools/GUI
quark-downloader --url 'https://www.youtube.com/playlist?list=...'  # playlist -> own folder
quark-downloader --print-default-output-dir
```

Run with no arguments for the interactive prompt flow.

## Env (Windows)

`QUARK_SKIP_YTDLP_UPDATE=1` | `QUARK_SKIP_FFMPEG_DOWNLOAD=1`
