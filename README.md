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

**YouTube:** Distro/apt yt-dlp is often too old. Prefer `pipx install yt-dlp` and [Node or Deno](https://github.com/yt-dlp/yt-dlp/wiki/EJS). Quark warns on stale versions and passes EJS flags when a JS runtime is on PATH.

**Build:** [Crystal](https://crystal-lang.org/) · [just](https://github.com/casey/just) · Windows installer: [Inno Setup 7](https://jrsoftware.org/isdl.php) + `installer/quark-downloader.iss`

## Commands

```bash
just run      # crystal run
just build    # release → build/ (UPX on Windows)
just clean
```

## Env (Windows)

`QUARK_SKIP_YTDLP_UPDATE=1` · `QUARK_SKIP_FFMPEG_DOWNLOAD=1`
