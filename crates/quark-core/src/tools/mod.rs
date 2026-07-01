pub mod ffmpeg;
pub mod js_runtime;
pub mod ytdlp;

/// A user-facing error from locating or fetching an external tool.
#[derive(Debug)]
pub struct ToolError(pub String);

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for ToolError {}

impl From<crate::net::http::HttpError> for ToolError {
    fn from(e: crate::net::http::HttpError) -> Self {
        ToolError(e.0)
    }
}
