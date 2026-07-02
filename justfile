set quiet := true

[default]
default:
    @just --list

# Build all crates in release mode.
[group('build')]
build:
    cargo build --workspace --release

# Run the CLI (pass args after `--`, e.g. `just run -- --url URL`).
[group('dev')]
run *args:
    cargo run -p quark-cli {{args}}

# Run the GUI (Slint backend by default).
[group('dev')]
run-gui:
    cargo run -p quark-downloader-gui

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
doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

# Build the GUI with a native backend feature, e.g. `just build-native native-cocoa`.
[group('build')]
build-native feature:
    cargo build -p quark-gui --features {{feature}}

[group('clean')]
clean:
    cargo clean
