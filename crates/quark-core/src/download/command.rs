//! Builds the yt-dlp argument vector for a single item.

use std::path::Path;

use super::progress::PROGRESS_TEMPLATE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaType {
    Audio,
    Video,
}

impl MediaType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MediaType::Audio => "audio",
            MediaType::Video => "video",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "audio" => Some(MediaType::Audio),
            "video" => Some(MediaType::Video),
            _ => None,
        }
    }
}

/// A format is "original" (no conversion) when it is empty or one of these.
pub fn is_original(format: &str) -> bool {
    let f = format.trim().to_ascii_lowercase();
    f.is_empty() || f == "original" || f == "default.original"
}

/// True when the requested conversion needs ffmpeg.
pub fn needs_ffmpeg(_media_type: MediaType, format: &str) -> bool {
    !is_original(format)
}

pub struct CommandParams<'a> {
    pub ytdlp: &'a Path,
    pub media_type: MediaType,
    pub format: &'a str,
    pub target_dir: &'a Path,
    pub is_playlist: bool,
    pub strip_video_ids: bool,
    /// Emit our machine-readable progress template (always on).
    pub structured_progress: bool,
    /// ffmpeg directory for `--ffmpeg-location`, when a conversion needs it.
    pub ffmpeg_dir: Option<&'a Path>,
    /// Extra args (e.g. YouTube JS runtime flags).
    pub extra_args: &'a [String],
}

/// Build the yt-dlp argv (excluding the URL, which the caller appends).
/// `argv[0]` is the yt-dlp path.
pub fn build_command(p: &CommandParams) -> Vec<String> {
    let format = p.format.trim().to_ascii_lowercase();
    let name_template = if p.strip_video_ids {
        "%(title)s.%(ext)s"
    } else {
        "%(title)s [%(id)s].%(ext)s"
    };
    let outtmpl = p
        .target_dir
        .join(name_template)
        .to_string_lossy()
        .to_string();

    let mut cmd: Vec<String> = vec![p.ytdlp.to_string_lossy().to_string()];

    if p.is_playlist {
        cmd.push("--yes-playlist".into());
        cmd.push("--ignore-errors".into());
    } else {
        cmd.push("--no-playlist".into());
    }
    cmd.push("-o".into());
    cmd.push(outtmpl);

    // A stalled connection must not block forever: turn a silent socket into an
    // error, with a few quick retries for transient blips. The stall watchdog
    // covers a yt-dlp that produces no output at all.
    cmd.extend(
        [
            "--socket-timeout",
            "30",
            "--retries",
            "3",
            "--fragment-retries",
            "3",
        ]
        .map(String::from),
    );

    let original = is_original(&format);
    match p.media_type {
        MediaType::Audio => {
            cmd.push("-f".into());
            cmd.push("bestaudio/best".into());
            if !original {
                push_ffmpeg_location(&mut cmd, p.ffmpeg_dir);
                cmd.push("-x".into());
                cmd.push("--audio-format".into());
                cmd.push(format.clone());
            }
        }
        MediaType::Video => {
            if !original {
                push_ffmpeg_location(&mut cmd, p.ffmpeg_dir);
                cmd.push("-f".into());
                cmd.push("bv*+ba/b".into());
                cmd.push("--merge-output-format".into());
                cmd.push(format.clone());
                match format.as_str() {
                    "webm" => {
                        cmd.push("--recode-video".into());
                        cmd.push("webm".into());
                    }
                    "mp4" => {
                        cmd.push("--remux-video".into());
                        cmd.push("mp4".into());
                    }
                    _ => {}
                }
            }
        }
    }

    if p.structured_progress {
        cmd.push("--newline".into());
        cmd.push("--no-color".into());
        cmd.push("--progress-template".into());
        cmd.push(PROGRESS_TEMPLATE.into());
    }

    cmd.extend(p.extra_args.iter().cloned());
    cmd
}

fn push_ffmpeg_location(cmd: &mut Vec<String>, dir: Option<&Path>) {
    if let Some(dir) = dir {
        cmd.push("--ffmpeg-location".into());
        cmd.push(dir.to_string_lossy().to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn params<'a>(
        media: MediaType,
        format: &'a str,
        dir: &'a PathBuf,
        extra: &'a [String],
        ff: Option<&'a Path>,
    ) -> CommandParams<'a> {
        CommandParams {
            ytdlp: Path::new("yt-dlp"),
            media_type: media,
            format,
            target_dir: dir,
            is_playlist: false,
            strip_video_ids: true,
            structured_progress: true,
            ffmpeg_dir: ff,
            extra_args: extra,
        }
    }

    #[test]
    fn original_video_skips_ffmpeg() {
        let dir = PathBuf::from("/out");
        let cmd = build_command(&params(MediaType::Video, "original", &dir, &[], None));
        assert!(!cmd.iter().any(|a| a == "--ffmpeg-location"));
        assert!(cmd.iter().any(|a| a == "--progress-template"));
        assert!(cmd.iter().any(|a| a == "--no-playlist"));
    }

    #[test]
    fn mp4_video_remuxes_with_ffmpeg() {
        let dir = PathBuf::from("/out");
        let ff = PathBuf::from("/usr/bin");
        let cmd = build_command(&params(MediaType::Video, "mp4", &dir, &[], Some(&ff)));
        assert!(cmd
            .windows(2)
            .any(|w| w[0] == "--remux-video" && w[1] == "mp4"));
        assert!(cmd
            .windows(2)
            .any(|w| w[0] == "--merge-output-format" && w[1] == "mp4"));
        assert!(cmd
            .windows(2)
            .any(|w| w[0] == "--ffmpeg-location" && w[1] == "/usr/bin"));
    }

    #[test]
    fn audio_extract_format() {
        let dir = PathBuf::from("/out");
        let ff = PathBuf::from("/usr/bin");
        let cmd = build_command(&params(MediaType::Audio, "mp3", &dir, &[], Some(&ff)));
        assert!(cmd.iter().any(|a| a == "-x"));
        assert!(cmd
            .windows(2)
            .any(|w| w[0] == "--audio-format" && w[1] == "mp3"));
    }
}
