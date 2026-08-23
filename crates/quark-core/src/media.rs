//! Typed media type and output format. CLI, engine, and GUI share this allow-list.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MediaType {
    Audio,
    #[default]
    Video,
}

impl MediaType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Audio => "audio",
            Self::Video => "video",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "audio" => Ok(Self::Audio),
            "video" => Ok(Self::Video),
            other => Err(format!(
                "Invalid media type: {other:?} (expected audio or video)"
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Format {
    #[default]
    Original,
    Mp3,
    M4a,
    Flac,
    Wav,
    Opus,
    Vorbis,
    Mp4,
    Mkv,
    Webm,
}

pub const AUDIO_FORMATS: &[&str] = &["original", "mp3", "m4a", "flac", "wav", "opus", "vorbis"];
pub const VIDEO_FORMATS: &[&str] = &["original", "mp4", "mkv", "webm"];

impl Format {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Original => "original",
            Self::Mp3 => "mp3",
            Self::M4a => "m4a",
            Self::Flac => "flac",
            Self::Wav => "wav",
            Self::Opus => "opus",
            Self::Vorbis => "vorbis",
            Self::Mp4 => "mp4",
            Self::Mkv => "mkv",
            Self::Webm => "webm",
        }
    }

    pub fn needs_ffmpeg(self) -> bool {
        self != Self::Original
    }

    pub fn choices(media: MediaType) -> &'static [&'static str] {
        match media {
            MediaType::Audio => AUDIO_FORMATS,
            MediaType::Video => VIDEO_FORMATS,
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match normalize_format_name(value).as_str() {
            "original" => Ok(Self::Original),
            "mp3" => Ok(Self::Mp3),
            "m4a" => Ok(Self::M4a),
            "flac" => Ok(Self::Flac),
            "wav" => Ok(Self::Wav),
            "opus" => Ok(Self::Opus),
            "vorbis" => Ok(Self::Vorbis),
            "mp4" => Ok(Self::Mp4),
            "mkv" => Ok(Self::Mkv),
            "webm" => Ok(Self::Webm),
            other => Err(format!(
                "Invalid format: {other:?} (expected one of {}, or {})",
                AUDIO_FORMATS.join("/"),
                VIDEO_FORMATS[1..].join("/")
            )),
        }
    }

    pub fn parse_for(media: MediaType, value: &str) -> Result<Self, String> {
        let format = Self::parse(value)?;
        if format == Self::Original || Self::choices(media).contains(&format.as_str()) {
            Ok(format)
        } else {
            Err(format!(
                "Format {:?} is not valid for {} (expected {})",
                format.as_str(),
                media.as_str(),
                Self::choices(media).join("/")
            ))
        }
    }
}

fn normalize_format_name(value: &str) -> String {
    let lower = value.trim().to_ascii_lowercase();
    match lower.as_str() {
        "" | "default.original" | "default" => "original".into(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_media_and_aliases() {
        assert_eq!(MediaType::parse("VIDEO").unwrap(), MediaType::Video);
        assert!(MediaType::parse("image").is_err());
        assert_eq!(Format::parse("").unwrap(), Format::Original);
        assert_eq!(Format::parse("default.original").unwrap(), Format::Original);
        assert_eq!(
            Format::parse_for(MediaType::Audio, "mp3").unwrap(),
            Format::Mp3
        );
        assert!(Format::parse_for(MediaType::Audio, "mp4").is_err());
        assert!(Format::parse_for(MediaType::Video, "avi").is_err());
        assert!(!Format::Original.needs_ffmpeg());
        assert!(Format::Mp4.needs_ffmpeg());
    }
}
