fn main() {
    #[cfg(windows)]
    windows_resources::embed("gui.rc", "gui");
}

#[cfg(windows)]
#[path = "../../build-support/windows_resources.rs"]
mod windows_resources;
