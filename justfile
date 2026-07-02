set quiet := true

# On Windows, run recipes with PowerShell instead of sh (recipes stay
# shell-agnostic: plain commands plus exported variables, no sh syntax).
set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

gui-features := if os() == "macos" {
    "quark-downloader-gui/native-cocoa"
} else if os() == "windows" {
    "quark-downloader-gui/native-windows"
} else if os() == "linux" {
    "quark-downloader-gui/native-gtk,quark-downloader-gui/native-kirigami"
} else {
    ""
}

[default]
default:
    @just --list

# Build all crates with every GUI backend supported by this platform.
[group('build')]
build:
    cargo build --workspace --release --features "{{gui-features}}"

# Run the CLI (pass args after `--`, e.g. `just run -- --url URL`).
[group('dev')]
run *args:
    cargo run -p quark-cli {{args}}

# Run the GUI with every backend supported by this platform.
[group('dev')]
run-gui:
    cargo run -p quark-downloader-gui --features "{{gui-features}}"

# Run the standalone QuarkGUI demo (proves the toolkit is reusable).
[group('dev')]
demo-gui:
    cargo run -p quark-gui --example standalone

# Run all tests.
[group('check')]
test:
    cargo test --workspace

# Lint with clippy (warnings as errors).
[group('check')]
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# Format the whole workspace.
[group('check')]
fmt:
    cargo fmt --all

# Build API docs (warnings are errors, matching CI).
[group('check')]
doc $RUSTDOCFLAGS="-D warnings":
    cargo doc --workspace --no-deps

[group('clean')]
clean:
    cargo clean
