//! The download engine: resolves tools, builds commands, runs yt-dlp, pumps
//! its output into [`ProgressEvent`]s, and recovers from stalls.

pub mod command;
pub mod naming;
pub mod playlist;
pub mod progress;
pub mod stall;
pub mod tracker;

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crossbeam_channel::{unbounded, Sender};

use crate::cancel::CancelToken;
use crate::config::Settings;
use crate::events::{EventSink, ProgressEvent};
use crate::paths;
use crate::tools::{ffmpeg, ytdlp};

use command::{build_command, CommandParams, MediaType};
use stall::{StallMonitor, STALL_TIMEOUT};
use tracker::DestinationTracker;

/// A complete request: one or more URLs and how to fetch them.
pub struct DownloadRequest {
    pub urls: Vec<String>,
    pub media_type: MediaType,
    pub format: String,
    pub output_dir: Option<PathBuf>,
    /// On Windows, spawn yt-dlp without a console window (GUI progress mode).
    pub hidden_console: bool,
}

/// Run a request to completion. Emits events to `sink` and honors `cancel`.
/// Returns the overall exit code.
pub fn run(
    request: &DownloadRequest,
    settings: &Settings,
    sink: &dyn EventSink,
    cancel: &CancelToken,
) -> i32 {
    let ytdlp_path = match ytdlp::ensure(settings.yt_dlp, sink) {
        Ok(p) => p,
        Err(e) => {
            sink.emit(ProgressEvent::Log(e.to_string()));
            sink.emit(ProgressEvent::Done { exit_code: 1 });
            return 1;
        }
    };
    ffmpeg::detect(settings.ffmpeg, sink);

    let output_path = request
        .output_dir
        .clone()
        .unwrap_or_else(|| settings.resolved_download_dir(&paths::default_downloads_dir()));
    if let Err(e) = std::fs::create_dir_all(&output_path) {
        sink.emit(ProgressEvent::Log(format!(
            "Error creating output directory:\n{e}"
        )));
        sink.emit(ProgressEvent::Done { exit_code: 1 });
        return 1;
    }

    let total = request.urls.len();
    let multi = total > 1;
    let mut failed: Vec<(String, i32)> = Vec::new();

    for (i, url) in request.urls.iter().enumerate() {
        if cancel.is_cancelled() {
            break;
        }
        if multi {
            sink.emit(ProgressEvent::QueuePosition {
                index: i + 1,
                total,
            });
            sink.emit(ProgressEvent::Log(format!(
                "\n==> URL {} of {}: {url}",
                i + 1,
                total
            )));
        }

        let code = match run_single(
            &ytdlp_path,
            url,
            request,
            &output_path,
            settings,
            sink,
            cancel,
        ) {
            Ok(code) => code,
            Err(message) => {
                sink.emit(ProgressEvent::Log(message));
                if multi {
                    1
                } else {
                    sink.emit(ProgressEvent::ItemFinished {
                        url: url.clone(),
                        exit_code: 1,
                    });
                    sink.emit(ProgressEvent::Done { exit_code: 1 });
                    return 1;
                }
            }
        };
        sink.emit(ProgressEvent::ItemFinished {
            url: url.clone(),
            exit_code: code,
        });
        if code != 0 {
            failed.push((url.clone(), code));
        }
    }

    finish(total, multi, &failed, sink)
}

fn finish(total: usize, multi: bool, failed: &[(String, i32)], sink: &dyn EventSink) -> i32 {
    if multi {
        sink.emit(ProgressEvent::Log(String::new()));
        sink.emit(ProgressEvent::Log(format!(
            "==> Finished: {} of {} succeeded.",
            total - failed.len(),
            total
        )));
        for (u, _) in failed {
            sink.emit(ProgressEvent::Log(format!("  failed: {u}")));
        }
        if failed.iter().any(|(u, _)| ytdlp::youtube_url(u)) {
            sink.emit(ProgressEvent::Log(String::new()));
            sink.emit(ProgressEvent::Log(ytdlp::youtube_failure_hints()));
        }
        let code = if failed.is_empty() { 0 } else { 1 };
        sink.emit(ProgressEvent::Done { exit_code: code });
        return code;
    }

    if failed.is_empty() {
        sink.emit(ProgressEvent::Log("Done.".into()));
        sink.emit(ProgressEvent::Done { exit_code: 0 });
        0
    } else {
        let (url, code) = &failed[0];
        let mut message = format!("Failed with exit code {code}.");
        if ytdlp::youtube_url(url) {
            message.push_str(&format!("\n\n{}", ytdlp::youtube_failure_hints()));
        }
        sink.emit(ProgressEvent::Log(message));
        sink.emit(ProgressEvent::Done { exit_code: *code });
        *code
    }
}

/// Download a single URL (which may itself be a playlist).
fn run_single(
    ytdlp_path: &Path,
    url: &str,
    request: &DownloadRequest,
    output_path: &Path,
    settings: &Settings,
    sink: &dyn EventSink,
    cancel: &CancelToken,
) -> Result<i32, String> {
    ytdlp::preflight_youtube(url).map_err(|e| e.to_string())?;

    let is_playlist = playlist::is_playlist_url(url);
    let extra = ytdlp::extra_args(url);
    let mut target_dir = output_path.to_path_buf();

    if is_playlist && settings.playlist_folders {
        if let Some(probe) = playlist::probe(ytdlp_path, url, &extra) {
            let folder = naming::sanitize_component(
                &probe.title,
                settings.sanitize_filenames,
                settings.spaces_policy(),
            );
            let candidate = output_path.join(&folder);
            match std::fs::create_dir_all(&candidate) {
                Ok(()) => {
                    target_dir = candidate.clone();
                    let note = probe
                        .count
                        .map(|c| format!(" ({c} items)"))
                        .unwrap_or_default();
                    sink.emit(ProgressEvent::Log(format!(
                        "Playlist: {}{}",
                        probe.title, note
                    )));
                    sink.emit(ProgressEvent::Log(format!(
                        "Saving into: {}",
                        target_dir.display()
                    )));
                }
                Err(e) => sink.emit(ProgressEvent::Log(format!(
                    "Warning: could not create playlist folder {}: {e}",
                    candidate.display()
                ))),
            }
        } else {
            sink.emit(ProgressEvent::Log(
                "Warning: could not read playlist info; downloading without a playlist folder."
                    .into(),
            ));
        }
    }

    let ffmpeg_dir = if command::needs_ffmpeg(request.media_type, &request.format) {
        Some(ffmpeg::ensure(settings.ffmpeg, sink, true).map_err(|e| e.to_string())?)
    } else {
        None
    };

    let params = CommandParams {
        ytdlp: ytdlp_path,
        media_type: request.media_type,
        format: &request.format,
        target_dir: &target_dir,
        is_playlist,
        strip_video_ids: settings.strip_video_ids,
        structured_progress: true,
        ffmpeg_dir: ffmpeg_dir.as_deref(),
        extra_args: &extra,
    };
    let base = build_command(&params);

    let mut tracker = DestinationTracker::new();
    let exit_code = if is_playlist {
        run_playlist(
            &base,
            url,
            &mut tracker,
            sink,
            cancel,
            request.hidden_console,
        )
    } else {
        let monitor = StallMonitor::new(0, None);
        let mut cmd = base.clone();
        cmd.push(url.to_string());
        run_command(
            &cmd,
            &mut tracker,
            &monitor,
            STALL_TIMEOUT,
            request.hidden_console,
            sink,
            cancel,
        )
    };

    for path in apply_naming(&tracker, output_path, settings, sink) {
        sink.emit(ProgressEvent::Log(format!("Saved: {}", path.display())));
    }
    if is_playlist && tracker.error_count() > 0 {
        sink.emit(ProgressEvent::Log(format!(
            "Playlist finished: {} item(s) failed.",
            tracker.error_count()
        )));
    }
    Ok(exit_code)
}

/// Run a playlist, restarting past any item silent for `STALL_TIMEOUT`.
fn run_playlist(
    base_cmd: &[String],
    url: &str,
    tracker: &mut DestinationTracker,
    sink: &dyn EventSink,
    cancel: &CancelToken,
    hidden: bool,
) -> i32 {
    let mut total: Option<u32> = None;
    let mut start: u32 = 1;
    let mut exit_code;

    loop {
        let mut cmd = base_cmd.to_vec();
        if start > 1 {
            cmd.push("--playlist-items".into());
            cmd.push(format!("{start}:"));
        }
        cmd.push(url.to_string());

        let monitor = StallMonitor::new(start - 1, total);
        exit_code = run_command(&cmd, tracker, &monitor, STALL_TIMEOUT, hidden, sink, cancel);
        if total.is_none() {
            total = monitor.total_items();
        }

        if !monitor.killed() || cancel.is_cancelled() {
            break;
        }

        let Some(item) = monitor.current_item() else {
            sink.emit(ProgressEvent::Log(
                "\nStopped: no response from the server.".into(),
            ));
            break;
        };
        sink.emit(ProgressEvent::Log(format!(
            "\nSkipping item {item}: no response for {}s.",
            STALL_TIMEOUT.as_secs()
        )));
        start = item + 1;
        if let Some(t) = total {
            if start > t {
                break;
            }
        }
    }
    exit_code
}

/// Spawn yt-dlp, pump its output, and watch for stalls/cancellation.
fn run_command(
    cmd: &[String],
    tracker: &mut DestinationTracker,
    monitor: &StallMonitor,
    stall_timeout: Duration,
    hidden: bool,
    sink: &dyn EventSink,
    cancel: &CancelToken,
) -> i32 {
    sink.emit(ProgressEvent::Debug(format!(
        "Running: {}",
        quote_command(cmd)
    )));

    let mut builder = Command::new(&cmd[0]);
    builder
        .args(&cmd[1..])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        if hidden {
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            builder.creation_flags(CREATE_NO_WINDOW);
        }
    }
    #[cfg(not(windows))]
    let _ = hidden;

    let mut child = match builder.spawn() {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            sink.emit(ProgressEvent::Log(format!(
                "Error: {} was not found.",
                cmd[0]
            )));
            return 127;
        }
        Err(e) => {
            sink.emit(ProgressEvent::Log(format!(
                "Error launching {}: {e}",
                cmd[0]
            )));
            return 127;
        }
    };

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let child = Arc::new(Mutex::new(child));
    let (tx, rx) = unbounded::<String>();

    let exit_status = std::thread::scope(|scope| {
        if let Some(out) = stdout {
            let tx = tx.clone();
            scope.spawn(move || pump_reader(out, tx));
        }
        if let Some(err) = stderr {
            let tx = tx.clone();
            scope.spawn(move || pump_reader(err, tx));
        }
        drop(tx); // rx closes once both readers finish

        // Watchdog: kill on stall or user cancellation.
        let watch_child = Arc::clone(&child);
        scope.spawn(move || loop {
            std::thread::sleep(Duration::from_secs(1));
            if monitor.finished() {
                break;
            }
            if cancel.is_cancelled() {
                if let Ok(mut c) = watch_child.lock() {
                    let _ = c.kill();
                }
                break;
            }
            if monitor.stalled(stall_timeout) {
                monitor.mark_killed();
                if let Ok(mut c) = watch_child.lock() {
                    let _ = c.kill();
                }
                break;
            }
        });

        // Main thread drains output (holding no child lock, so the watchdog can
        // always acquire it to kill).
        while let Ok(line) = rx.recv() {
            process_line(line, monitor, tracker, sink);
        }

        let status = child.lock().unwrap().wait();
        monitor.finish();
        status
    });

    match exit_status {
        Ok(status) => exit_code_of(&status),
        Err(_) => 127,
    }
}

fn process_line(
    raw: String,
    monitor: &StallMonitor,
    tracker: &mut DestinationTracker,
    sink: &dyn EventSink,
) {
    let (line, playlist_item) = monitor.observe(&raw);
    tracker.observe(&line);

    if let Some(prog) = progress::parse_progress(&line) {
        sink.emit(ProgressEvent::Progress {
            percent: prog.percent,
        });
        sink.emit(ProgressEvent::Eta(prog.eta));
        return;
    }

    if let Some((item, total)) = playlist_item {
        sink.emit(ProgressEvent::PlaylistItem {
            item,
            total: Some(total),
        });
    }
    sink.emit(ProgressEvent::Log(line.clone()));
    if let Some(status) = progress::status_line(&line) {
        sink.emit(ProgressEvent::Status(status));
    }
}

/// Read a stream, splitting tokens on both `\n` and `\r` (yt-dlp emits
/// carriage-return progress), sending each token to `tx`.
fn pump_reader<R: Read>(reader: R, tx: Sender<String>) {
    let mut reader = BufReader::new(reader);
    let mut buf: Vec<u8> = Vec::new();
    loop {
        let chunk = match reader.fill_buf() {
            Ok([]) | Err(_) => break,
            Ok(chunk) => chunk,
        };
        let len = chunk.len();
        let mut consumed = 0;
        for (i, &c) in chunk.iter().enumerate() {
            if c == b'\n' || c == b'\r' {
                buf.extend_from_slice(&chunk[consumed..i]);
                let _ = tx.send(String::from_utf8_lossy(&buf).into_owned());
                buf.clear();
                consumed = i + 1;
            }
        }
        buf.extend_from_slice(&chunk[consumed..]);
        reader.consume(len);
    }
    if !buf.is_empty() {
        let _ = tx.send(String::from_utf8_lossy(&buf).into_owned());
    }
}

/// Rename downloaded files per the naming settings. Only touches files yt-dlp
/// reported, that still exist, inside the output directory. Failures are
/// logged and never abort. Returns the final paths of the surviving files
/// (renamed or not) so callers can report what was saved.
fn apply_naming(
    tracker: &DestinationTracker,
    output_path: &Path,
    settings: &Settings,
    sink: &dyn EventSink,
) -> Vec<PathBuf> {
    let policy = settings.spaces_policy();
    let renaming = settings.sanitize_filenames || !matches!(policy, naming::SpacesPolicy::Keep);
    let base = std::fs::canonicalize(output_path).unwrap_or_else(|_| output_path.to_path_buf());
    let mut saved = Vec::new();

    for path in tracker.paths() {
        if path.ends_with(".part") || path.ends_with(".ytdl") {
            continue;
        }
        let expanded = match std::fs::canonicalize(path) {
            Ok(p) => p,
            Err(_) => continue,
        };
        if expanded != base && !expanded.starts_with(&base) {
            continue;
        }
        if !expanded.is_file() {
            continue;
        }
        let (Some(dir), Some(name)) = (
            expanded.parent(),
            expanded.file_name().and_then(|n| n.to_str()),
        ) else {
            continue;
        };
        if !renaming {
            saved.push(expanded.clone());
            continue;
        }
        let new_name = naming::sanitize_filename(name, settings.sanitize_filenames, policy);
        if new_name == name {
            saved.push(expanded.clone());
            continue;
        }
        let Some(final_name) = naming::collision_free(dir, &new_name) else {
            saved.push(expanded.clone());
            continue;
        };
        match std::fs::rename(&expanded, dir.join(&final_name)) {
            Ok(()) => {
                sink.emit(ProgressEvent::Debug(format!(
                    "Renamed: {name} -> {final_name}"
                )));
                saved.push(dir.join(&final_name));
            }
            Err(e) => {
                sink.emit(ProgressEvent::Log(format!(
                    "Warning: could not rename {path}: {e}"
                )));
                saved.push(expanded.clone());
            }
        }
    }
    saved
}

fn quote_command(cmd: &[String]) -> String {
    cmd.iter()
        .map(|x| {
            if x.contains(' ') {
                format!("\"{x}\"")
            } else {
                x.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(unix)]
fn exit_code_of(status: &std::process::ExitStatus) -> i32 {
    use std::os::unix::process::ExitStatusExt;
    status
        .code()
        .or_else(|| status.signal().map(|s| 128 + s))
        .unwrap_or(127)
}

#[cfg(not(unix))]
fn exit_code_of(status: &std::process::ExitStatus) -> i32 {
    status.code().unwrap_or(127)
}
