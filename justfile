set quiet

# On Windows, run recipes with PowerShell instead of sh (recipes stay
# shell-agnostic: plain commands plus exported variables, no sh syntax).
set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

gui-features := if os() == "macos" { "quark-downloader-gui/native-cocoa,quark-downloader-gui/native-gtk,quark-downloader-gui/native-kirigami" } else if os() == "windows" { "quark-downloader-gui/native-windows,quark-downloader-gui/native-kirigami" } else if os() == "linux" { "quark-downloader-gui/native-gtk,quark-downloader-gui/native-kirigami" } else { "" }
toolkit-features := if os() == "macos" { "native-cocoa,native-gtk,native-kirigami" } else if os() == "windows" { "native-windows,native-kirigami" } else if os() == "linux" { "native-gtk,native-kirigami" } else { "" }
pkg-config-command := if os() == "macos" { env_var_or_default("PKG_CONFIG", shell("brew --prefix pkgconf") + "/bin/pkg-config") } else { env_var_or_default("PKG_CONFIG", "pkg-config") }
export PKG_CONFIG := pkg-config-command

windows-target := "x86_64-pc-windows-gnu"
windows-gui := "target/" + windows-target + "/debug/quark-downloader-gui.exe"
windows-cli := "target/" + windows-target + "/debug/quark-downloader.exe"
wine-command := env_var_or_default("WINE", "wine")
wine-prefix := justfile_directory() + "/target/wine"
wine-debug := env_var_or_default("WINEDEBUG", "-all")
mvk-log-level := env_var_or_default("MVK_CONFIG_LOG_LEVEL", "0")

[default]
default:
    @just --list

# Build all crates with every GUI backend supported by this platform.
[group('build')]
build:
    cargo build --workspace --release --features "{{ gui-features }}"

# Run the CLI (pass args after `--`, e.g. `just run -- --url URL`).
[group('dev')]
run *args:
    cargo run -p quark-cli {{ args }}

# Run the GUI with every backend supported by this platform.
[group('dev')]
run-gui:
    cargo run -p quark-downloader-gui --features "{{ gui-features }}"

# Cross-compile the complete Windows CLI and GUI on macOS or Linux.
[group('windows')]
[unix]
windows-build: _windows-build-prereqs
    cargo build --workspace --target "{{ windows-target }}" --features "quark-downloader-gui/native-windows"

# Cross-compile and execute the Windows test binaries through Wine.
[group('windows')]
[unix]
windows-test: _windows-wine-prereqs
    CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUNNER="{{ wine-command }}" WINEPREFIX="{{ wine-prefix }}" WINEDEBUG="{{ wine-debug }}" MVK_CONFIG_LOG_LEVEL="{{ mvk-log-level }}" cargo test --workspace --target "{{ windows-target }}" --features "quark-downloader-gui/native-windows"

# Launch the cross-compiled Windows GUI through Wine (Win32 is the default).
[group('windows')]
[unix]
windows-run-gui: windows-build _windows-wine-prereqs
    WINEPREFIX="{{ wine-prefix }}" WINEDEBUG="{{ wine-debug }}" MVK_CONFIG_LOG_LEVEL="{{ mvk-log-level }}" "{{ wine-command }}" "{{ windows-gui }}"

# Launch the cross-compiled Windows CLI through Wine.
[group('windows')]
[unix]
windows-run-cli *args: windows-build _windows-wine-prereqs
    WINEPREFIX="{{ wine-prefix }}" WINEDEBUG="{{ wine-debug }}" MVK_CONFIG_LOG_LEVEL="{{ mvk-log-level }}" "{{ wine-command }}" "{{ windows-cli }}" {{ args }}

# Run the standalone QuarkGUI demo (proves the toolkit is reusable).
[group('dev')]
demo-gui:
    cargo run -p quark-gui --example standalone --features "{{ toolkit-features }}"

# Run all tests.
[group('check')]
test:
    cargo test --workspace --features "{{ gui-features }}"

# Lint with clippy (warnings as errors).
[group('check')]
lint:
    cargo clippy --workspace --all-targets --features "{{ gui-features }}" -- -D warnings

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

[private]
[unix]
_windows-build-prereqs:
    @rustup target list --installed | grep -qx "{{ windows-target }}" || { echo "Missing Rust target. Run: rustup target add {{ windows-target }}"; exit 1; }
    @command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1 || { echo "Missing MinGW-w64 compiler. See the Windows cross-testing section in README.md."; exit 1; }

[private]
[unix]
_windows-wine-prereqs:
    @command -v "{{ wine-command }}" >/dev/null 2>&1 || { echo "Missing Wine executable '{{ wine-command }}'. Install Wine or set WINE=/path/to/wine."; exit 1; }
