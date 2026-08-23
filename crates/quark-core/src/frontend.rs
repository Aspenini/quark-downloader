use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::config::{GuiTheme, Settings};
use crate::result::DownloadResult;
use crate::session::{self, MainSessionResult};

#[derive(Debug)]
pub struct FrontendError(pub String);

impl std::fmt::Display for FrontendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for FrontendError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageKind {
    Ok,
    Error,
}

impl MessageKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Error => "error",
        }
    }
}

pub trait ProgressSink {
    fn send(&mut self, cmd: &crate::progress::ProgressCmd) -> Result<(), FrontendError>;
    fn wait_closed(&mut self) -> bool;
}

pub trait Frontend {
    fn collect_session(&self, default_dir: &str, settings: &Settings) -> MainSessionResult;
    fn show_message(&self, kind: MessageKind, title: &str, body: &str);
    fn show_completion(&self, result: &DownloadResult);
    fn open_progress(&self, theme: GuiTheme) -> Result<Box<dyn ProgressSink>, FrontendError>;
}

pub const LINUX_AUTO_ORDER: &[&str] = &["qt"];
pub const MACOS_AUTO_ORDER: &[&str] = &["appkit"];

pub fn linux_auto_order() -> &'static [&'static str] {
    LINUX_AUTO_ORDER
}

/// Frontends compiled into quark-downloader-gui and re-exec'd with `--frontend`.
pub fn is_builtin_frontend(id: &str) -> bool {
    id == "qt" || id == "appkit"
}

pub fn host_auto_order() -> &'static [&'static str] {
    if quark_platform::prefers_appkit() {
        MACOS_AUTO_ORDER
    } else {
        linux_auto_order()
    }
}

pub fn helper_binary_name(id: &str) -> String {
    format!("quark-downloader-gui-{id}")
}

pub fn discover_helper(builtins: &[&str]) -> Result<(String, PathBuf), FrontendError> {
    if quark_platform::uses_inprocess_gui() {
        return Err(FrontendError(
            "Win32 in-process frontend does not use a helper binary".into(),
        ));
    }
    if let Some(env) = std::env::var_os("QUARK_GUI_FRONTEND") {
        let value = env.to_string_lossy();
        let path = PathBuf::from(value.as_ref());
        if path.exists() {
            return Ok((value.into_owned(), path));
        }
        if let Some(found) = lookup_id(value.as_ref(), builtins) {
            return Ok((value.into_owned(), found));
        }
        return Err(FrontendError(format!(
            "QUARK_GUI_FRONTEND={value} was set but no helper was found."
        )));
    }
    for id in host_auto_order() {
        if let Some(path) = lookup_id(id, builtins) {
            return Ok(((*id).to_string(), path));
        }
    }
    Err(missing_helper_error(host_auto_order().first().copied()))
}

fn lookup_id(id: &str, builtins: &[&str]) -> Option<PathBuf> {
    if builtins.contains(&id) {
        return std::env::current_exe().ok();
    }
    lookup_named(&helper_binary_name(id))
}

fn lookup_named(name: &str) -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        let sibling = parent.join(name);
        if is_executable(&sibling) {
            return Some(sibling);
        }
    }
    let dev = PathBuf::from("build").join(name);
    if is_executable(&dev) {
        return Some(dev);
    }
    let target = PathBuf::from("target").join("release").join(name);
    if is_executable(&target) {
        return Some(target);
    }
    crate::process::which(name)
}

fn is_executable(path: &Path) -> bool {
    path.is_file()
}

fn missing_helper_error(id: Option<&str>) -> FrontendError {
    let hint = match id {
        Some("qt") => {
            "Qt frontend is not available.\nInstall Qt 6 Declarative and rebuild quark-downloader-gui."
                .into()
        }
        Some("appkit") => "AppKit frontend is not available in this build.".into(),
        Some(id) => format!("GUI frontend '{id}' is not available in this build."),
        None => "No GUI frontend is available in this build.".into(),
    };
    FrontendError(hint)
}

pub struct HelperFrontend {
    pub id: String,
    pub path: PathBuf,
}

impl HelperFrontend {
    pub fn discover(builtins: &[&str]) -> Result<Self, FrontendError> {
        let (id, path) = discover_helper(builtins)?;
        Ok(Self { id, path })
    }

    fn uses_frontend_flag(&self) -> bool {
        is_builtin_frontend(&self.id)
            || std::env::current_exe().is_ok_and(|exe| exe == self.path)
    }

    fn run(&self, args: &[String]) -> Result<(i32, String), FrontendError> {
        eprintln!("Opening {} ({})", self.id, self.path.display());
        // Builtin frontends live in this binary. Re-exec with --frontend so they
        // get a dedicated process and can write session JSON.
        let mut cmd = if self.uses_frontend_flag() {
            let mut c = Command::new(std::env::current_exe().unwrap_or_else(|_| self.path.clone()));
            c.arg("--frontend").arg(&self.id);
            c
        } else {
            Command::new(&self.path)
        };
        let output = cmd
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|e| FrontendError(e.to_string()))?;
        let code = crate::process::exit_code(Some(output.status), 1);
        let text = String::from_utf8_lossy(&output.stdout).into_owned();
        Ok((code, text))
    }
}

impl Frontend for HelperFrontend {
    fn collect_session(&self, default_dir: &str, settings: &Settings) -> MainSessionResult {
        let args = session::build_session_args(default_dir, settings);
        match self.run(&args) {
            Ok((0, text)) => session::parse(&text),
            Ok((code, text)) => {
                let parsed = session::parse(&text);
                if matches!(parsed.action, crate::session::MainAction::Error(_)) {
                    parsed
                } else {
                    crate::session::MainSessionResult::error(format!(
                        "GUI helper exited with status {code}"
                    ))
                }
            }
            Err(e) => crate::session::MainSessionResult::error(e.0),
        }
    }

    fn show_message(&self, kind: MessageKind, title: &str, body: &str) {
        let _ = self.run(&[
            "--message".into(),
            kind.as_str().into(),
            title.into(),
            body.into(),
        ]);
    }

    fn show_completion(&self, result: &DownloadResult) {
        if result.success() {
            self.show_message(
                MessageKind::Ok,
                crate::version::APP_NAME,
                &format!("Download complete!\n\n{}", result.dialog_body()),
            );
        } else {
            let mut body = result.dialog_body();
            if body.trim().is_empty() {
                body = "Download failed.".into();
            }
            self.show_message(MessageKind::Error, crate::version::APP_NAME, &body);
        }
    }

    fn open_progress(&self, theme: GuiTheme) -> Result<Box<dyn ProgressSink>, FrontendError> {
        let mut cmd = if self.uses_frontend_flag() {
            let mut c = Command::new(std::env::current_exe().unwrap_or_else(|_| self.path.clone()));
            c.arg("--frontend").arg(&self.id);
            c
        } else {
            Command::new(&self.path)
        };
        let mut child = cmd
            .args(["--progress", "", theme.as_str()])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| FrontendError(e.to_string()))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| FrontendError("progress helper has no stdin".into()))?;
        Ok(Box::new(HelperProgress {
            child,
            stdin,
            closed: false,
        }))
    }
}

struct HelperProgress {
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    closed: bool,
}

impl ProgressSink for HelperProgress {
    fn send(&mut self, cmd: &crate::progress::ProgressCmd) -> Result<(), FrontendError> {
        use std::io::Write;
        writeln!(self.stdin, "{}", cmd.encode()).map_err(|e| FrontendError(e.to_string()))?;
        self.stdin.flush().map_err(|e| FrontendError(e.to_string()))
    }

    fn wait_closed(&mut self) -> bool {
        if self.closed {
            return true;
        }
        match self.child.try_wait() {
            Ok(Some(_)) => {
                self.closed = true;
                true
            }
            _ => false,
        }
    }
}

impl Drop for HelperProgress {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub fn last_resort_error(message: &str) {
    eprintln!("{message}");
    for tool in ["zenity", "kdialog", "notify-send"] {
        if crate::process::which(tool).is_some() {
            let _ = match tool {
                "zenity" => Command::new(tool)
                    .args(["--error", "--text", message])
                    .status(),
                "kdialog" => Command::new(tool).args(["--error", message]).status(),
                "notify-send" => Command::new(tool)
                    .args(["Quark Downloader", message])
                    .status(),
                _ => continue,
            };
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helper_names() {
        assert_eq!(helper_binary_name("appkit"), "quark-downloader-gui-appkit");
        assert_eq!(LINUX_AUTO_ORDER, &["qt"]);
        assert_eq!(MACOS_AUTO_ORDER, &["appkit"]);
        assert!(is_builtin_frontend("qt"));
        assert!(is_builtin_frontend("appkit"));
        assert!(!is_builtin_frontend("win32"));
    }

    #[test]
    fn env_path_override_missing_is_error() {
        // SAFETY: test process is single-threaded here.
        unsafe {
            std::env::set_var("QUARK_GUI_FRONTEND", "/no/such/quark-frontend-binary");
        }
        let err = discover_helper(&[]);
        unsafe {
            std::env::remove_var("QUARK_GUI_FRONTEND");
        }
        assert!(err.is_err());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn builtin_id_resolves_to_current_exe() {
        let (id, path) = discover_helper(&["qt"]).unwrap();
        assert_eq!(id, "qt");
        assert_eq!(path, std::env::current_exe().unwrap());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn builtin_id_resolves_to_current_exe() {
        let (id, path) = discover_helper(&["appkit"]).unwrap();
        assert_eq!(id, "appkit");
        assert_eq!(path, std::env::current_exe().unwrap());
    }
}
