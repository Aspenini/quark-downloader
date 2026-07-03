//! Quark Downloader engine: configuration, tool management, and the yt-dlp
//! download orchestration. Front-ends (CLI, GUI) consume this crate and never
//! print directly — they observe [`events::ProgressEvent`]s instead.

pub mod cancel;
pub mod config;
pub mod download;
pub mod events;
pub mod logs;
pub mod net;
pub mod paths;
pub mod release_check;
pub mod tools;
pub mod util;
pub mod version;
pub mod version_compare;

pub use cancel::CancelToken;
pub use config::Settings;
pub use download::command::MediaType;
pub use download::{run, DownloadRequest};
pub use events::{EventSink, ProgressEvent};
