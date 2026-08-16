# GUI dispatcher and frontend protocol

`quark-downloader-gui` never links a toolkit. It discovers a named helper and
speaks this protocol. New Linux frontends (COSMIC, Kirigami, …) only need to
implement the three modes below and install as `quark-downloader-gui-<id>`.

## Discovery

1. `QUARK_GUI_FRONTEND` — id or full path
2. Config `gui_frontend` (`auto`, `gtk`, `cosmic`, `kirigami`)
3. Sibling of the dispatcher, then `PATH`

`auto` on Linux tries `gtk`, then `cosmic`, then `kirigami`.
macOS prefers `quark-downloader-gui-appkit` (legacy `quark-downloader-gui-helper` is still accepted).
Windows uses in-process Win32.

## `--session`

Positional argv after `--session`:

```
<default_dir> <download_dir> <yt_dlp> <ffmpeg> <gui_download_mode>
<download_logs> <gui_theme> <strip_video_ids> <sanitize_filenames>
<filename_spaces> <playlist_folders> [gui_frontend]
```

Print one JSON object on stdout:

```json
{"v":1,"action":"download","settings":{...},"urls":["..."],"media_type":"video","format":"original","output_dir":"..."}
```

`action` is `download` or `cancel`. Legacy `__SESSION__` lines are still parsed.

## `--progress <unused> <theme>`

Read stdin commands: `PROGRESS\t`, `STATUS\t`, `ETA\t`, `QUEUE\t`, `DONE\t<code>`.
Closing the window cancels the download.

## `--message <ok|error> <title> <body>`

Show an alert and exit.
