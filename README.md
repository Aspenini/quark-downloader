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
          <img alt="Version 0.2.0" src="https://img.shields.io/badge/version-0.2.0-blue" />
        </a>
        <a href="https://aur.archlinux.org/packages/quark-downloader">
          <img alt="AUR version" src="https://img.shields.io/aur/version/quark-downloader?label=AUR" />
        </a>
      </p>
    </td>
  </tr>
</table>

## Dependencies

| | Windows | macOS / Linux |
|---|---------|----------------|
| **yt-dlp** | PATH or auto-download to `tools/` | PATH — keep it current for YouTube |
| **ffmpeg** | PATH, `bundled-tools/` → `build/tools/` on build, or auto-download | PATH (`brew install ffmpeg`, etc.) |
| **GUI (optional)** | Built-in Win32 dialog | [Tk](https://www.tcl.tk/) / `wish` (`apt install tk`, `brew install tcl-tk`) |

**YouTube:** Distro/apt yt-dlp is often too old. Prefer `pipx install yt-dlp` and [Node or Deno](https://github.com/yt-dlp/yt-dlp/wiki/EJS). Quark warns on stale versions and passes EJS flags when a JS runtime is on PATH.

**Build:** [Crystal](https://crystal-lang.org/) · [just](https://github.com/casey/just) · Windows installer: [Inno Setup 7](https://jrsoftware.org/isdl.php) + `packaging/quark-downloader.iss`

## Binaries

| Program | Purpose |
|---------|---------|
| `quark-downloader` | Full CLI — interactive in a terminal, or scriptable with flags |
| `quark-downloader-gui` | Thin UI that collects options and runs the CLI as a subprocess |

Package maintainers can ship the CLI alone (`quark-downloader` on PATH) and optionally a GUI package that installs `quark-downloader-gui`, `quark-downloader-gui.tcl` (same directory), [`packaging/quark-downloader-gui.desktop`](packaging/quark-downloader-gui.desktop), and depends on **Tk** / `wish` (Linux/macOS).

Windows shortcuts from the installer open the GUI; the CLI remains in the install folder as **Quark Downloader (CLI)**.

## Commands

```bash
just run          # crystal run CLI
just run-gui      # crystal run GUI
just build        # release → build/ (both binaries; UPX on Windows)
just clean
```

### CLI (non-interactive)

```bash
quark-downloader --url 'https://…' --type video --format mp4 --output-dir ~/Downloads --no-pause
quark-downloader --print-default-output-dir
```

Run with no arguments for the interactive prompt flow.

## Env (Windows)

`QUARK_SKIP_YTDLP_UPDATE=1` · `QUARK_SKIP_FFMPEG_DOWNLOAD=1`
