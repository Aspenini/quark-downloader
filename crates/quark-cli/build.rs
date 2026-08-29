fn main() {
    #[cfg(windows)]
    windows_resources::embed("app.rc", "icon-cli");
}

#[cfg(windows)]
#[path = "../../build-support/windows_resources.rs"]
mod windows_resources;
