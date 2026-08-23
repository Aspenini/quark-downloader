# Scripts

Prefer `just` recipes for day-to-day use. The scripts are grouped by platform so
build helpers stay close to the platform they support.

## Public Entry Points

Windows:

- `windows/build.ps1`
- `windows/run-gui.ps1`
- `windows/run-android.ps1` (`just android-run`)
- `windows/clean.ps1`

Unix/macOS:

- `unix/build.sh`
- `unix/clean.sh`

## Windows Helpers

- `windows/common.ps1` - shared path helpers.
- `windows/copy-bundled-tools.ps1` - copies bundled ffmpeg tools into `build/tools`.
