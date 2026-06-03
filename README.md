<table width="100%">
  <tr>
    <td align="left" width="120">
      <img src="icons/icon.png" alt="Quark Downloader" width="100" />
    </td>
    <td align="right">
      <h1>Quark Downloader</h1>
    </td>
  </tr>
</table>

## Dependencies

| | Windows | macOS / Linux |
|---|---------|----------------|
| **yt-dlp** | PATH or auto-download to `tools/` | PATH — keep it current for YouTube |
| **ffmpeg** | PATH, `bundled-tools/` → `build/tools/` on build, or auto-download | PATH (`brew install ffmpeg`, etc.) |
| **GUI (optional)** | Built-in Win32 dialog | [zenity](https://github.com/nco/zenity) (`apt install zenity`, `brew install zenity`) |

**YouTube:** Distro/apt yt-dlp is often too old. Prefer `pipx install yt-dlp` and [Node or Deno](https://github.com/yt-dlp/yt-dlp/wiki/EJS). Quark warns on stale versions and passes EJS flags when a JS runtime is on PATH.

**Build:** [Crystal](https://crystal-lang.org/) · [just](https://github.com/casey/just) · Windows installer: [Inno Setup 7](https://jrsoftware.org/isdl.php) + `packaging/quark-downloader.iss`

## Binaries

| Program | Purpose |
|---------|---------|
| `quark-downloader` | Full CLI — interactive in a terminal, or scriptable with flags |
| `quark-downloader-gui` | Thin UI that collects options and runs the CLI as a subprocess |

Package maintainers can ship the CLI alone (`quark-downloader` on PATH) and optionally a GUI package that installs `quark-downloader-gui`, [`packaging/quark-downloader-gui.desktop`](packaging/quark-downloader-gui.desktop), and depends on **zenity** (Linux/macOS).

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
