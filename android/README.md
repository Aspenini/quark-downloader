# Android

Jetpack Compose host for Quark Downloader. This is **not** a port of the desktop
CLI. yt-dlp runs through [youtubedl-android](https://github.com/yausername/youtubedl-android)
0.18.1 (embedded CPython + ffmpeg + QuickJS). The APK that links that library
is GPL-3.0; desktop crates stay MIT.

Share a link to Quark (or open a YouTube URL with it) to enqueue. Finished files
go to the system **Downloads** folder.

## Kill criterion

On a physical **arm64** device:

1. Version
2. Download a YouTube URL (EJS / n-challenge must succeed)
3. MP3 extract (ffmpeg)
4. File appears under the app-specific Downloads folder shown on screen

If that fails, stop. Do not invent a Python embed.

## Build

Need JDK 17 and Android SDK (`ANDROID_HOME` or `android/local.properties`).

```bash
just android-spike    # debug APK
just android-run      # boot emulator, install, launch
```

`just android-run` builds arm64-v8a + x86_64, starts AVD **Quark** (android-35 google_apis x86_64, 4 KB pages) if needed, then `adb install` + launches the spike. Override the AVD with `ANDROID_AVD`. The existing Pixel 9 Pro image is 16 KB pages and often cannot load youtubedl-android's Python.

First-time Quark AVD (if `just android-run` has not created it):

```bash
sdkmanager "system-images;android-35;google_apis;x86_64"
```

Debug APK: `android/app/build/outputs/apk/debug/app-debug.apk`.

Legacy JNI packaging (`useLegacyPackaging`) is required so Python and ffmpeg
are executable from `nativeLibraryDir`.

## Shared engine

The APK loads `libquark.so` (`crates/quark-android`). Catalog, session reducer,
yt-dlp argv, playlist detection, filename sanitize, and progress parse all live
in Rust. `just android-run` / Gradle `preBuild` cross-compiles it for arm64-v8a
and x86_64 via `scripts/windows/build-android-jni.ps1` (needs NDK 26+).

```bash
cargo test -p quark-android
```
