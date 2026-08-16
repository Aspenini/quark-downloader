//! Shared download engine, config, and frontend protocol for Quark Downloader.

pub mod color;
pub mod config;
pub mod destination;
pub mod download;
pub mod ffmpeg;
pub mod filename;
pub mod frontend;
pub mod http;
pub mod json;
pub mod logs;
pub mod playlist;
pub mod process;
pub mod progress;
pub mod release;
pub mod result;
pub mod session;
pub mod url;
pub mod version;
pub mod version_cmp;
pub mod ytdlp;

pub use config::{ConfigError, Settings};
pub use result::DownloadResult;
pub use version::{APP_NAME, VERSION};
