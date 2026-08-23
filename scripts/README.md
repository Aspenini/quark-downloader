# Scripts

Prefer `just` recipes for day-to-day use. The scripts are grouped by platform so
build helpers stay close to the platform they support.

## Public Entry Points

Windows:

- `windows/build.ps1`
- `windows/run-gui.ps1`
- `windows/run-android.ps1` (`just android-run`)
- `windows/release-android.ps1` (`just android-release`)
- `windows/build-android-jni.ps1`
- `windows/clean.ps1`

Unix/macOS:

- `unix/build.sh`
- `unix/clean.sh`
- `unix/run-android.sh` (`just android-run`)
- `unix/release-android.sh` (`just android-release`)
- `unix/build-android-jni.sh`

Shared:

- `align_elf_16k.py` — 16 KB ELF page alignment for Android `.so` files

## Windows Helpers

- `windows/common.ps1` - shared path helpers.
- `windows/copy-bundled-tools.ps1` - copies bundled ffmpeg tools into `build/tools`.
