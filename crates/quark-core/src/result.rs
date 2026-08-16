use crate::json::{self, Value};

pub const RESULT_PREFIX: &str = "__RESULT__";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DownloadResult {
    pub exit_code: i32,
    pub output_dir: String,
    pub files: Vec<String>,
    pub errors: Vec<String>,
    pub failed_urls: Vec<String>,
    pub log_path: Option<String>,
    pub playlist_error_count: i32,
}

impl DownloadResult {
    pub fn success(&self) -> bool {
        self.exit_code == 0 && self.failed_urls.is_empty()
    }

    pub fn to_json(&self) -> String {
        let files = self
            .files
            .iter()
            .map(|f| json::stringify_str(f))
            .collect::<Vec<_>>()
            .join(",");
        let errors = self
            .errors
            .iter()
            .map(|e| json::stringify_str(e))
            .collect::<Vec<_>>()
            .join(",");
        let failed = self
            .failed_urls
            .iter()
            .map(|u| json::stringify_str(u))
            .collect::<Vec<_>>()
            .join(",");
        let log = match &self.log_path {
            Some(p) => json::stringify_str(p),
            None => "null".into(),
        };
        format!(
            "{{\"exit_code\":{},\"output_dir\":{},\"files\":[{files}],\"errors\":[{errors}],\"failed_urls\":[{failed}],\"log_path\":{log},\"playlist_error_count\":{}}}",
            self.exit_code,
            json::stringify_str(&self.output_dir),
            self.playlist_error_count
        )
    }

    pub fn to_emit_line(&self) -> String {
        format!("{RESULT_PREFIX}{}", self.to_json())
    }

    pub fn parse_emit_line(line: &str) -> Option<Self> {
        let stripped = line.trim();
        let json = stripped.strip_prefix(RESULT_PREFIX)?;
        let value = json::parse(json).ok()?;
        Some(Self::from_json(&value))
    }

    pub fn from_json(value: &Value) -> Self {
        let files = value
            .get("files")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect();
        let errors = value
            .get("errors")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect();
        let failed_urls = value
            .get("failed_urls")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect();
        let log_path = value.get_str("log_path").map(str::to_string);
        Self {
            exit_code: value.get_i32("exit_code").unwrap_or(0),
            output_dir: value.get_str("output_dir").unwrap_or("").to_string(),
            files,
            errors,
            failed_urls,
            log_path,
            playlist_error_count: value.get_i32("playlist_error_count").unwrap_or(0),
        }
    }

    pub fn dialog_body(&self) -> String {
        self.dialog_body_limited(8, 6)
    }

    pub fn dialog_body_limited(&self, max_files: usize, max_errors: usize) -> String {
        let mut parts = Vec::new();
        if !self.output_dir.is_empty() {
            parts.push(format!("Folder: {}", self.output_dir));
        }
        if self.success() {
            if self.files.is_empty() {
                parts.push(
                    "Look for new files in that folder (names may have been sanitized).".into(),
                );
            } else {
                parts.push("Saved:".into());
                for f in self.files.iter().take(max_files) {
                    parts.push(format!("  {f}"));
                }
                if self.files.len() > max_files {
                    parts.push(format!("  … and {} more", self.files.len() - max_files));
                }
            }
        } else {
            if !self.errors.is_empty() {
                parts.push("Errors:".into());
                let start = self.errors.len().saturating_sub(max_errors);
                for e in &self.errors[start..] {
                    parts.push(format!("  {e}"));
                }
            } else if !self.failed_urls.is_empty() {
                parts.push("Failed URL(s):".into());
                for u in self.failed_urls.iter().take(max_errors) {
                    parts.push(format!("  {u}"));
                }
            } else {
                parts.push(format!("Download failed (exit code {}).", self.exit_code));
            }
            if let Some(path) = &self.log_path {
                parts.push(String::new());
                parts.push(format!("Log: {path}"));
            }
        }
        if self.playlist_error_count > 0 {
            parts.push(String::new());
            parts.push(format!(
                "Playlist items failed: {}",
                self.playlist_error_count
            ));
        }
        parts.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_and_parses_emit_lines() {
        let result = DownloadResult {
            exit_code: 0,
            output_dir: "/tmp/out".into(),
            files: vec!["/tmp/out/a.mp4".into()],
            log_path: Some("/tmp/log.txt".into()),
            ..DownloadResult::default()
        };
        let line = result.to_emit_line();
        assert!(line.starts_with(RESULT_PREFIX));
        let parsed = DownloadResult::parse_emit_line(&line).unwrap();
        assert_eq!(parsed.exit_code, 0);
        assert_eq!(parsed.output_dir, "/tmp/out");
        assert_eq!(parsed.files, vec!["/tmp/out/a.mp4"]);
        assert_eq!(parsed.log_path.as_deref(), Some("/tmp/log.txt"));
        assert!(parsed.success());
    }

    #[test]
    fn builds_failure_dialog_body() {
        let result = DownloadResult {
            exit_code: 1,
            output_dir: "/tmp/out".into(),
            errors: vec!["ERROR: boom".into()],
            failed_urls: vec!["https://example.com".into()],
            log_path: Some("/tmp/x.log".into()),
            ..DownloadResult::default()
        };
        let body = result.dialog_body();
        assert!(body.contains("ERROR: boom"));
        assert!(body.contains("/tmp/x.log"));
        assert!(!result.success());
    }

    #[test]
    fn ignores_non_result_lines() {
        assert!(DownloadResult::parse_emit_line("PROGRESS\t50").is_none());
    }
}
