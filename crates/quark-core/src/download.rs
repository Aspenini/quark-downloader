use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::color;
use crate::config::{self, ConfigError, Settings};
use crate::destination::DestinationTracker;
use crate::ffmpeg;
use crate::filename;
use crate::logs;
use crate::playlist;
use crate::process;
use crate::result::DownloadResult;
use crate::version_cmp;
use crate::ytdlp;

pub const STALL_GRACE: Duration = Duration::from_secs(90);
pub const STALL_ACTIVE: Duration = Duration::from_secs(75);

#[derive(Debug)]
pub struct PreflightError(pub String);

impl std::fmt::Display for PreflightError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for PreflightError {}

struct SingleRunOutcome {
    exit_code: i32,
    files: Vec<String>,
    errors: Vec<String>,
    target_dir: PathBuf,
    playlist_error_count: i32,
}

pub fn default_downloads_dir() -> PathBuf {
    #[cfg(not(windows))]
    if let Some(xdg) = xdg_download_dir() {
        return xdg;
    }
    config::user_home().join("Downloads")
}

#[cfg(not(windows))]
fn xdg_download_dir() -> Option<PathBuf> {
    let home = config::user_home();
    let file = home.join(".config").join("user-dirs.dirs");
    let text = fs::read_to_string(file).ok()?;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("XDG_DOWNLOAD_DIR=\"")
            && let Some(inner) = rest.strip_suffix('"')
        {
            let expanded = inner.replace("$HOME", &home.to_string_lossy());
            return Some(config::expand_path(&expanded));
        }
    }
    None
}

pub fn default_output_dir() -> PathBuf {
    let settings = config::load(true).unwrap_or_default();
    settings.download_dir_expanded(&default_downloads_dir())
}

pub fn stall_timeout_from_env(default: Duration) -> Duration {
    if let Ok(raw) = std::env::var("QUARK_STALL_TIMEOUT_SEC")
        && let Ok(secs) = raw.parse::<u64>()
        && secs > 0
    {
        return Duration::from_secs(secs);
    }
    default
}

pub fn run(
    url: &str,
    media_type: &str,
    format: &str,
    output_dir: Option<&Path>,
    no_pause: bool,
    emit_result: bool,
) -> i32 {
    run_all(
        &[url.to_string()],
        media_type,
        format,
        output_dir,
        no_pause,
        emit_result,
    )
}

pub fn run_all(
    urls: &[String],
    media_type: &str,
    format: &str,
    output_dir: Option<&Path>,
    no_pause: bool,
    emit_result: bool,
) -> i32 {
    let result = execute(urls, media_type, format, output_dir, no_pause);
    let emit = emit_result
        || std::env::var_os("QUARK_GUI").as_deref() == Some(std::ffi::OsStr::new("1"))
        || std::env::var_os("QUARK_EMIT_RESULT").as_deref() == Some(std::ffi::OsStr::new("1"));
    if emit {
        emit_result_line(&result);
    }
    result.exit_code
}

pub fn execute(
    urls: &[String],
    media_type: &str,
    format: &str,
    output_dir: Option<&Path>,
    no_pause: bool,
) -> DownloadResult {
    let mut result = DownloadResult::default();
    let settings = match config::load(true) {
        Ok(s) => s,
        Err(ConfigError(msg)) => {
            result.exit_code = 1;
            result.errors.push(msg.clone());
            eprintln!("{}", color::red(&msg));
            return result;
        }
    };

    logs::open_download_log(settings.download_logs);
    result.log_path = logs::active_path().map(|p| p.to_string_lossy().into_owned());
    warn_if_root();
    warn_if_unwritable_config();

    let outcome = (|| {
        let media_type = media_type.to_ascii_lowercase();
        if media_type != "audio" && media_type != "video" {
            return fail_result(
                result,
                &format!("Invalid media type: {media_type:?} (expected audio or video)"),
                no_pause,
                1,
            );
        }
        let mut format = format.to_ascii_lowercase();
        if format.is_empty() {
            format = "original".into();
        }
        let dir = output_dir
            .map(Path::to_path_buf)
            .unwrap_or_else(|| settings.download_dir_expanded(&default_downloads_dir()));
        let output_path = if dir.is_absolute() {
            dir
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(dir)
        };
        result.output_dir = output_path.to_string_lossy().into_owned();

        let ytdlp = match preflight(&settings, urls, &media_type, &format, &output_path) {
            Ok(p) => p,
            Err(msg) => return fail_result(result, &msg, no_pause, 1),
        };

        let multi = urls.len() > 1;
        let mut failed: Vec<(String, i32)> = Vec::new();

        for (index, url) in urls.iter().enumerate() {
            if multi {
                logs::log_line(&format!(
                    "\n{}: {url}",
                    color::bold(&format!("==> URL {} of {}", index + 1, urls.len()))
                ));
            }
            let outcome =
                match run_single(&settings, &ytdlp, url, &media_type, &format, &output_path) {
                    Ok(o) => o,
                    Err(msg) if multi => {
                        logs::log_line(&color::red(&msg));
                        SingleRunOutcome {
                            exit_code: 1,
                            files: Vec::new(),
                            errors: vec![msg],
                            target_dir: output_path.clone(),
                            playlist_error_count: 0,
                        }
                    }
                    Err(msg) => return fail_result(result, &msg, no_pause, 1),
                };
            result.files.extend(outcome.files);
            result.errors.extend(outcome.errors);
            result.playlist_error_count += outcome.playlist_error_count;
            if urls.len() == 1 {
                result.output_dir = outcome.target_dir.to_string_lossy().into_owned();
            }
            if outcome.exit_code != 0 {
                failed.push((url.clone(), outcome.exit_code));
                result.failed_urls.push(url.clone());
            }
        }

        result.files.sort();
        result.files.dedup();
        result.errors.sort();
        result.errors.dedup();

        if multi {
            logs::log_line("");
            let ok = urls.len() - failed.len();
            let summary = format!("==> Finished: {ok} of {} succeeded.", urls.len());
            logs::log_line(&if failed.is_empty() {
                color::green(&summary)
            } else {
                color::yellow(&summary)
            });
            for (u, _) in &failed {
                logs::log_line(&color::red(&format!("  failed: {u}")));
            }
            if failed.iter().any(|(u, _)| ytdlp::youtube_url(u)) {
                logs::log_line("");
                logs::log_line(&ytdlp::youtube_failure_hints());
            }
            press_any_key(no_pause, "Press any key to exit...");
            result.exit_code = if failed.is_empty() { 0 } else { 1 };
            return result;
        }

        if failed.is_empty() {
            logs::log_line(&color::green("Done."));
            press_any_key(no_pause, "Press any key to exit...");
            result.exit_code = 0;
            result
        } else {
            let (_, code) = failed[0];
            let mut message = format!("Failed with exit code {code}.");
            if ytdlp::youtube_url(&failed[0].0) {
                message.push_str("\n\n");
                message.push_str(&ytdlp::youtube_failure_hints());
            }
            fail_result(result, &message, no_pause, code)
        }
    })();

    logs::close();
    outcome
}

pub fn preflight(
    settings: &Settings,
    urls: &[String],
    media_type: &str,
    format: &str,
    output_path: &Path,
) -> Result<PathBuf, String> {
    let _ = media_type;
    fs::create_dir_all(output_path).map_err(|ex| {
        format!(
            "Cannot create output directory:\n  {}\n{ex}",
            output_path.display()
        )
    })?;
    if !dir_writable(output_path) {
        return Err(format!(
            "Output directory is not writable:\n  {}\nChoose another folder (do not use sudo to \"fix\" permissions).",
            output_path.display()
        ));
    }
    let ytdlp = ytdlp::ensure(settings).map_err(|e| e.0)?;
    let needs_ffmpeg = format != "original" && format != "default.original";
    if needs_ffmpeg {
        ffmpeg::ensure(settings).map_err(|e| e.0)?;
    } else {
        ffmpeg::detect(settings);
    }
    for url in urls {
        ytdlp::preflight_youtube(url).map_err(|e| e.0)?;
    }
    if urls.iter().any(|u| ytdlp::youtube_url(u))
        && let Some(version) = ytdlp::read_version(&ytdlp)
        && !version_cmp::at_least(&version, ytdlp::MIN_YOUTUBE_YTDLP)
    {
        logs::log_line(&color::yellow(&format!(
            "Warning: yt-dlp {version} is likely too old for YouTube (want >= {}).",
            ytdlp::MIN_YOUTUBE_YTDLP
        )));
    }
    Ok(ytdlp)
}

fn dir_writable(dir: &Path) -> bool {
    if !dir.is_dir() {
        return false;
    }
    let probe = dir.join(format!(".quark-write-test-{}", std::process::id()));
    match fs::write(&probe, "ok") {
        Ok(()) => {
            let _ = fs::remove_file(&probe);
            true
        }
        Err(_) => {
            let _ = fs::remove_file(&probe);
            false
        }
    }
}

fn run_single(
    settings: &Settings,
    ytdlp: &Path,
    url: &str,
    media_type: &str,
    format: &str,
    output_path: &Path,
) -> Result<SingleRunOutcome, String> {
    let is_playlist = playlist::playlist_url(url);
    let mut target_dir = output_path.to_path_buf();
    if is_playlist && settings.playlist_folders {
        if let Some(probe) = playlist::probe(&ytdlp.to_string_lossy(), url, &ytdlp::extra_args(url))
        {
            let folder = filename::sanitize_component(
                &probe.title,
                settings.sanitize_filenames,
                settings.filename_spaces.to_policy(),
            );
            let candidate = output_path.join(folder);
            match fs::create_dir_all(&candidate) {
                Ok(()) => {
                    target_dir = candidate;
                    let count_note = probe
                        .count
                        .map(|c| format!(" ({c} items)"))
                        .unwrap_or_default();
                    logs::log_line(&format!("Playlist: {}{count_note}", probe.title));
                    logs::log_line(&format!(
                        "Saving into: {}",
                        color::cyan(&target_dir.to_string_lossy())
                    ));
                }
                Err(ex) => logs::log_line(&color::yellow(&format!(
                    "Warning: could not create playlist folder {}: {ex}",
                    candidate.display()
                ))),
            }
        } else {
            logs::log_line(&color::yellow(
                "Warning: could not read playlist info; downloading without a playlist folder.",
            ));
        }
    }

    let name_template = if settings.strip_video_ids {
        "%(title)s.%(ext)s"
    } else {
        "%(title)s [%(id)s].%(ext)s"
    };
    let outtmpl = target_dir.join(name_template);

    let mut cmd = vec![ytdlp.to_string_lossy().into_owned()];
    if is_playlist {
        cmd.extend(["--yes-playlist".into(), "--ignore-errors".into()]);
    } else {
        cmd.push("--no-playlist".into());
    }
    cmd.extend(["-o".into(), outtmpl.to_string_lossy().into_owned()]);
    cmd.extend([
        "--socket-timeout".into(),
        "30".into(),
        "--retries".into(),
        "3".into(),
        "--fragment-retries".into(),
        "3".into(),
    ]);

    if media_type == "audio" {
        cmd.extend(["-f".into(), "bestaudio/best".into()]);
        if format != "original" && format != "default.original" {
            ffmpeg::append_to_cmd(&mut cmd, settings).map_err(|e| e.0)?;
            cmd.extend(["-x".into(), "--audio-format".into(), format.to_string()]);
        }
    } else if format != "original" && format != "default.original" {
        ffmpeg::append_to_cmd(&mut cmd, settings).map_err(|e| e.0)?;
        cmd.extend([
            "-f".into(),
            "bv*+ba/b".into(),
            "--merge-output-format".into(),
            format.to_string(),
        ]);
        match format {
            "webm" => cmd.extend(["--recode-video".into(), "webm".into()]),
            "mp4" => cmd.extend(["--remux-video".into(), "mp4".into()]),
            _ => {}
        }
    }

    if std::env::var_os("QUARK_GUI").as_deref() == Some(std::ffi::OsStr::new("1")) {
        cmd.extend(["--newline".into(), "--no-color".into()]);
    }
    cmd.extend(ytdlp::extra_args(url));

    let tracker = DestinationTracker::new();
    let active_timeout = stall_timeout_from_env(STALL_ACTIVE);
    let grace_timeout = stall_timeout_from_env(STALL_GRACE);

    let exit_code = if is_playlist {
        run_playlist(&cmd, url, &tracker, active_timeout, grace_timeout)
    } else {
        let monitor = StallMonitor::new(0, None, false);
        let mut full = cmd.clone();
        full.push(url.into());
        run_command(
            &full,
            Some(&tracker),
            Some(&monitor),
            Some(active_timeout),
            Some(grace_timeout),
        )
    };

    let final_paths = apply_naming(&tracker, output_path, settings);
    if is_playlist && tracker.error_count() > 0 {
        logs::log_line(&color::yellow(&format!(
            "Playlist finished: {} item(s) failed.",
            tracker.error_count()
        )));
    }
    if exit_code == 0 || !final_paths.is_empty() {
        report_saved_files(&final_paths, &target_dir);
    }

    Ok(SingleRunOutcome {
        exit_code,
        files: final_paths,
        errors: tracker.errors(),
        target_dir,
        playlist_error_count: tracker.error_count() as i32,
    })
}

fn report_saved_files(paths: &[String], target_dir: &Path) {
    logs::log_line(&format!(
        "{} {}",
        color::bold("Output folder:"),
        color::cyan(&target_dir.to_string_lossy())
    ));
    let existing: Vec<&String> = paths
        .iter()
        .filter(|path| {
            !path.ends_with(".part")
                && !path.ends_with(".ytdl")
                && Path::new(path.as_str()).is_file()
        })
        .collect();
    if existing.is_empty() {
        logs::log_line(&color::dim(
            "Look for new files under that folder (names may have been sanitized).",
        ));
    } else {
        logs::log_line(&color::bold("Saved file(s):"));
        for p in existing {
            logs::log_line(&format!("  {}", color::cyan(p)));
        }
    }
}

fn warn_if_root() {
    #[cfg(unix)]
    if crate::sys::unix::is_root() {
        logs::log_line(&color::yellow("Warning: running as root/sudo."));
        logs::log_line(&color::yellow(&format!(
            "  Config and downloads use root's home ({}), not your user account.",
            config::user_home().display()
        )));
        logs::log_line(&color::yellow(
            "  Re-run without sudo so files land in your Downloads.",
        ));
    }
}

fn warn_if_unwritable_config() {
    #[cfg(unix)]
    {
        if crate::sys::unix::is_root() {
            return;
        }
        let path = config::config_dir();
        if !path.is_dir() {
            return;
        }
        let probe = path.join(format!(".quark-write-test-{}", std::process::id()));
        if fs::write(&probe, "ok").is_err() {
            logs::log_line(&color::yellow("Warning: config directory is not writable:"));
            logs::log_line(&color::yellow(&format!("  {}", path.display())));
            logs::log_line(&color::yellow(
                "  If you previously ran with sudo, fix ownership (do not keep using sudo):",
            ));
            logs::log_line(&color::yellow(&format!(
                "  sudo chown -R \"$USER\" {}",
                path.display()
            )));
        } else {
            let _ = fs::remove_file(probe);
        }
    }
}

fn apply_naming(
    tracker: &DestinationTracker,
    output_path: &Path,
    settings: &Settings,
) -> Vec<String> {
    let policy = settings.filename_spaces.to_policy();
    let base = fs::canonicalize(output_path).unwrap_or_else(|_| output_path.to_path_buf());
    let mut finals = Vec::new();
    for path in tracker.paths() {
        if path.ends_with(".part") || path.ends_with(".ytdl") {
            continue;
        }
        let expanded = fs::canonicalize(&path).unwrap_or_else(|_| PathBuf::from(&path));
        let base_s = base.to_string_lossy();
        let exp_s = expanded.to_string_lossy();
        let sep = std::path::MAIN_SEPARATOR;
        if exp_s != base_s && !exp_s.starts_with(&format!("{base_s}{sep}")) {
            continue;
        }
        if !expanded.is_file() {
            continue;
        }
        if !settings.sanitize_filenames && policy == filename::SpacesPolicy::Keep {
            finals.push(expanded.to_string_lossy().into_owned());
            continue;
        }
        let dir = expanded.parent().unwrap_or(Path::new("."));
        let name = expanded
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let new_name = filename::sanitize_filename(&name, settings.sanitize_filenames, policy);
        if new_name == name {
            finals.push(expanded.to_string_lossy().into_owned());
            continue;
        }
        let Some(final_name) = filename::collision_free(dir, &new_name) else {
            finals.push(expanded.to_string_lossy().into_owned());
            continue;
        };
        let dest = dir.join(&final_name);
        match fs::rename(&expanded, &dest) {
            Ok(()) => {
                logs::log_line(&format!("Renamed: {name} -> {final_name}"));
                finals.push(dest.to_string_lossy().into_owned());
            }
            Err(ex) => logs::log_line(&color::yellow(&format!(
                "Warning: could not rename {path}: {ex}"
            ))),
        }
    }
    finals
}

pub struct StallMonitor {
    last: Mutex<Instant>,
    started: Instant,
    had_output: Mutex<bool>,
    suspended: Mutex<bool>,
    killed: Mutex<bool>,
    finished: Mutex<bool>,
    warned: Mutex<bool>,
    current_item: Mutex<Option<i32>>,
    total_items: Mutex<Option<i32>>,
    offset: i32,
    kill_on_stall: bool,
}

impl StallMonitor {
    pub fn new(offset: i32, total_items: Option<i32>, kill_on_stall: bool) -> Self {
        let now = Instant::now();
        Self {
            last: Mutex::new(now),
            started: now,
            had_output: Mutex::new(false),
            suspended: Mutex::new(false),
            killed: Mutex::new(false),
            finished: Mutex::new(false),
            warned: Mutex::new(false),
            current_item: Mutex::new(None),
            total_items: Mutex::new(total_items),
            offset,
            kill_on_stall,
        }
    }

    pub fn observe(&self, line: &str) -> String {
        if let Ok(mut last) = self.last.lock() {
            *last = Instant::now();
        }
        if let Ok(mut had) = self.had_output.lock() {
            *had = true;
        }
        if let Some((item, total)) = parse_playlist_item_line(line) {
            if let Ok(mut t) = self.total_items.lock() {
                *t = Some(t.unwrap_or(total + self.offset));
            }
            let abs = item + self.offset;
            if let Ok(mut c) = self.current_item.lock() {
                *c = Some(abs);
            }
            if let Ok(mut s) = self.suspended.lock() {
                *s = false;
            }
            let shown_total = self
                .total_items
                .lock()
                .ok()
                .and_then(|t| *t)
                .unwrap_or(total);
            return format!("[download] Downloading item {abs} of {shown_total}");
        }
        if is_postprocess_line(line) {
            if let Ok(mut s) = self.suspended.lock() {
                *s = true;
            }
        } else if is_resume_line(line) || is_progress_hint(line) {
            if let Ok(mut s) = self.suspended.lock() {
                *s = false;
            }
        }
        line.to_string()
    }

    pub fn stalled(&self, active: Duration, grace: Duration) -> bool {
        if self.suspended.lock().map(|s| *s).unwrap_or(false)
            || self.finished.lock().map(|f| *f).unwrap_or(false)
        {
            return false;
        }
        let had = self.had_output.lock().map(|h| *h).unwrap_or(false);
        let timeout = if had { active } else { grace };
        let anchor = if had {
            self.last.lock().map(|l| *l).unwrap_or(self.started)
        } else {
            self.started
        };
        Instant::now().duration_since(anchor) >= timeout
    }

    pub fn kill_on_stall(&self) -> bool {
        self.kill_on_stall
    }
    pub fn mark_killed(&self) {
        if let Ok(mut k) = self.killed.lock() {
            *k = true;
        }
    }
    pub fn mark_warned(&self) {
        if let Ok(mut w) = self.warned.lock() {
            *w = true;
        }
    }
    pub fn warned(&self) -> bool {
        self.warned.lock().map(|w| *w).unwrap_or(false)
    }
    pub fn finish(&self) {
        if let Ok(mut f) = self.finished.lock() {
            *f = true;
        }
    }
    pub fn killed(&self) -> bool {
        self.killed.lock().map(|k| *k).unwrap_or(false)
    }
    pub fn current_item(&self) -> Option<i32> {
        self.current_item.lock().ok().and_then(|c| *c)
    }
    pub fn total_items(&self) -> Option<i32> {
        self.total_items.lock().ok().and_then(|t| *t)
    }
}

fn parse_playlist_item_line(line: &str) -> Option<(i32, i32)> {
    let rest = line.strip_prefix("[download] Downloading item ")?;
    let mut parts = rest.split_whitespace();
    let item = parts.next()?.parse().ok()?;
    if parts.next() != Some("of") {
        return None;
    }
    let total = parts.next()?.parse().ok()?;
    Some((item, total))
}

fn is_postprocess_line(line: &str) -> bool {
    const TAGS: &[&str] = &[
        "[Merger]",
        "[ExtractAudio]",
        "[VideoConvertor]",
        "[VideoRemuxer]",
        "[Recode]",
        "[Metadata]",
        "[EmbedSubtitle]",
        "[EmbedThumbnail]",
        "[SponsorBlock]",
        "[ModifyChapters]",
        "[SplitChapters]",
    ];
    TAGS.iter().any(|t| line.starts_with(t)) || line.starts_with("[Fixup")
}

fn is_resume_line(line: &str) -> bool {
    line.starts_with("[download]") || line.contains("Extracting URL")
}

fn is_progress_hint(line: &str) -> bool {
    line.contains("Downloading item") || line.contains("Destination:") || line.contains('%')
}

fn run_playlist(
    opts: &[String],
    url: &str,
    tracker: &DestinationTracker,
    active: Duration,
    grace: Duration,
) -> i32 {
    let mut total = None;
    let mut start = 1;
    let mut exit_code;
    loop {
        let mut cmd = opts.to_vec();
        if start > 1 {
            cmd.extend(["--playlist-items".into(), format!("{start}:")]);
        }
        cmd.push(url.into());
        let monitor = StallMonitor::new(start - 1, total, true);
        exit_code = run_command(
            &cmd,
            Some(tracker),
            Some(&monitor),
            Some(active),
            Some(grace),
        );
        if total.is_none() {
            total = monitor.total_items();
        }
        if !monitor.killed() {
            break;
        }
        let Some(item) = monitor.current_item() else {
            logs::log_line(&color::yellow("\nStopped: no response from the server."));
            break;
        };
        logs::log_line(&color::yellow(&format!(
            "\nSkipping item {item}: no response for {}s.",
            active.as_secs()
        )));
        start = item + 1;
        if let Some(t) = total
            && start > t
        {
            break;
        }
    }
    exit_code
}

fn run_command(
    cmd: &[String],
    tracker: Option<&DestinationTracker>,
    monitor: Option<&StallMonitor>,
    active: Option<Duration>,
    grace: Option<Duration>,
) -> i32 {
    #[cfg(windows)]
    if std::env::var_os("QUARK_GUI").as_deref() == Some(std::ffi::OsStr::new("1")) {
        return run_command_hidden(cmd, tracker, monitor, active, grace);
    }

    logs::log_line("");
    logs::log_line(&color::dim("Running:"));
    logs::log_line(&color::dim(&quote_cmd(cmd)));
    logs::log_line("");

    let Some((prog, args)) = cmd.split_first() else {
        return 127;
    };
    let mut child = match Command::new(prog)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            logs::log_line(&color::red(&format!("Error: {prog} was not found.")));
            return 127;
        }
        Err(e) => {
            logs::log_line(&color::red(&format!("Error: {e}")));
            return 127;
        }
    };

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let child = Arc::new(Mutex::new(child));
    let child_watch = Arc::clone(&child);

    thread::scope(|s| {
        if let Some(pipe) = stdout {
            s.spawn(|| relay_pipe(pipe, false, tracker, monitor));
        }
        if let Some(pipe) = stderr {
            s.spawn(|| relay_pipe(pipe, true, tracker, monitor));
        }
        if let (Some(monitor), Some(active), Some(grace)) = (monitor, active, grace) {
            s.spawn(move || {
                loop {
                    thread::sleep(Duration::from_secs(1));
                    if monitor.finished.lock().map(|f| *f).unwrap_or(true) {
                        break;
                    }
                    if monitor.stalled(active, grace) {
                        if monitor.kill_on_stall() {
                            monitor.mark_killed();
                            if let Ok(mut child) = child_watch.lock() {
                                let _ = child.kill();
                            }
                            break;
                        } else if !monitor.warned() {
                            monitor.mark_warned();
                            logs::log_line(&color::yellow(
                                "\nWarning: no response for a while; still waiting…",
                            ));
                        }
                    }
                }
            });
        }

        let status = child.lock().ok().and_then(|mut c| c.wait().ok());
        if let Some(m) = monitor {
            m.finish();
        }
        process::exit_code(status, 127)
    })
}

fn relay_pipe(
    reader: impl io::Read,
    is_err: bool,
    tracker: Option<&DestinationTracker>,
    monitor: Option<&StallMonitor>,
) {
    use std::io::BufRead;
    let mut lines = io::BufReader::new(reader).lines();
    while let Some(Ok(line)) = lines.next() {
        let out_line = monitor
            .map(|m| m.observe(&line))
            .unwrap_or_else(|| line.clone());
        if let Some(t) = tracker {
            t.observe(&out_line);
        }
        if is_err {
            logs::log_line_err(&out_line);
        } else {
            logs::log_line(&out_line);
        }
    }
}

fn quote_cmd(cmd: &[String]) -> String {
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

fn emit_result_line(result: &DownloadResult) {
    println!("{}", result.to_emit_line());
    let _ = io::stdout().flush();
}

fn fail_result(
    mut result: DownloadResult,
    message: &str,
    no_pause: bool,
    code: i32,
) -> DownloadResult {
    logs::log_line(&color::red(message));
    if !result.errors.iter().any(|e| e == message) {
        result.errors.push(message.to_string());
    }
    result.exit_code = code;
    press_any_key(no_pause, "Press any key to exit...");
    result
}

pub fn press_any_key(no_pause: bool, message: &str) {
    if no_pause {
        return;
    }
    #[cfg(windows)]
    {
        logs::log_line("");
        logs::log_line(message);
        let mut buf = [0u8; 1];
        let _ = io::stdin().read_exact(&mut buf);
    }
    #[cfg(not(windows))]
    {
        let _ = message;
    }
}

#[cfg(windows)]
fn run_command_hidden(
    cmd: &[String],
    tracker: Option<&DestinationTracker>,
    monitor: Option<&StallMonitor>,
    active: Option<Duration>,
    grace: Option<Duration>,
) -> i32 {
    let Some((prog, args)) = cmd.split_first() else {
        return 127;
    };
    let runner = match crate::sys::windows::HiddenProcess::spawn(prog, args) {
        Ok(r) => r,
        Err(_) => {
            logs::log_line(&format!("Error: {prog} was not found."));
            return 127;
        }
    };
    let stdout = runner.stdout_handle() as usize;
    let stderr = runner.stderr_handle() as usize;
    thread::scope(|s| {
        s.spawn(|| {
            crate::sys::windows::read_handle_lines(stdout as *mut core::ffi::c_void, |line| {
                let out_line = monitor
                    .map(|m| m.observe(line))
                    .unwrap_or_else(|| line.to_string());
                if let Some(t) = tracker {
                    t.observe(&out_line);
                }
                logs::log_line(&out_line);
            });
        });
        s.spawn(|| {
            crate::sys::windows::read_handle_lines(stderr as *mut core::ffi::c_void, |line| {
                let out_line = monitor
                    .map(|m| m.observe(line))
                    .unwrap_or_else(|| line.to_string());
                if let Some(t) = tracker {
                    t.observe(&out_line);
                }
                logs::log_line_err(&out_line);
            });
        });
        if let (Some(monitor), Some(active), Some(grace)) = (monitor, active, grace) {
            loop {
                if runner.wait_ms(1000).is_some() {
                    break;
                }
                if monitor.stalled(active, grace) {
                    if monitor.kill_on_stall() {
                        monitor.mark_killed();
                        runner.terminate();
                        break;
                    } else if !monitor.warned() {
                        monitor.mark_warned();
                        logs::log_line("\nWarning: no response for a while; still waiting…");
                    }
                }
            }
        }
        let status = runner.wait();
        if let Some(m) = monitor {
            m.finish();
        }
        process::exit_code(None, status as i32)
    })
}
