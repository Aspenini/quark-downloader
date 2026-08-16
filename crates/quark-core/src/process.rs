use std::path::{Path, PathBuf};
use std::process::ExitStatus;

pub fn exit_code(status: Option<ExitStatus>, fallback: i32) -> i32 {
    match status {
        Some(s) if s.success() => 0,
        Some(s) => s.code().unwrap_or(fallback),
        None => fallback,
    }
}

pub fn which(name: &str) -> Option<PathBuf> {
    #[cfg(windows)]
    {
        crate::sys::windows::which(name).or_else(|| which_path(name))
    }
    #[cfg(not(windows))]
    {
        which_path(name)
    }
}

fn which_path(name: &str) -> Option<PathBuf> {
    let name_path = Path::new(name);
    if name_path.components().count() > 1 {
        return existing_exe(name_path);
    }
    let paths = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&paths) {
        if let Some(found) = existing_exe(&dir.join(name)) {
            return Some(found);
        }
        #[cfg(windows)]
        {
            if !name.ends_with(".exe")
                && let Some(found) = existing_exe(&dir.join(format!("{name}.exe")))
            {
                return Some(found);
            }
        }
    }
    None
}

fn existing_exe(path: &Path) -> Option<PathBuf> {
    if path.is_file() {
        Some(path.to_path_buf())
    } else {
        None
    }
}

#[cfg(windows)]
pub fn spawn_cmd_start_wait(title: &str, command: &str, args: &[String]) -> i32 {
    crate::sys::windows::spawn_cmd_start_wait(title, command, args)
}

#[cfg(windows)]
pub struct HiddenProcess(crate::sys::windows::HiddenProcess);

#[cfg(windows)]
unsafe impl Send for HiddenProcess {}
#[cfg(windows)]
unsafe impl Sync for HiddenProcess {}

#[cfg(windows)]
impl HiddenProcess {
    pub fn spawn(command: &str, args: &[String]) -> std::io::Result<Self> {
        crate::sys::windows::HiddenProcess::spawn(command, args).map(Self)
    }

    pub fn stdout_handle(&self) -> *mut core::ffi::c_void {
        self.0.stdout_handle()
    }

    pub fn stderr_handle(&self) -> *mut core::ffi::c_void {
        self.0.stderr_handle()
    }

    pub fn wait(&self) -> u32 {
        self.0.wait()
    }

    pub fn try_wait(&self) -> Option<u32> {
        self.0.wait_ms(0)
    }

    pub fn terminate(&self) {
        self.0.terminate();
    }
}

#[cfg(windows)]
pub fn read_handle_lines(handle: *mut core::ffi::c_void, on_line: impl FnMut(&str)) {
    crate::sys::windows::read_handle_lines(handle, on_line);
}
