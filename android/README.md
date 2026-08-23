# Android (spike)

Jetpack Compose host for Quark Downloader. This is **not** a port of the desktop
CLI. yt-dlp runs through [youtubedl-android](https://github.com/yausername/youtubedl-android)
0.18.1 (embedded CPython + ffmpeg + QuickJS). The APK that links that library
is GPL-3.0; desktop crates stay MIT.

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
cd android
./gradlew :app:assembleDebug
```

Install `app/build/outputs/apk/debug/app-debug.apk`. arm64-v8a only.

Legacy JNI packaging (`useLegacyPackaging`) is required so Python and ffmpeg
are executable from `nativeLibraryDir`.

## Shared engine (next)

`crates/quark-android` is the JNI cdylib (`libquark.so`): reducer script,
yt-dlp argv, progress parse, filename sanitize. Host tests:

```bash
cargo test -p quark-android
```

Cross-compile after `cargo install cargo-ndk` (NDK 26+ on `ANDROID_NDK_HOME`):

```bash
cargo ndk -t arm64-v8a -o android/app/src/main/jniLibs build -p quark-android --release
```

Do not `System.loadLibrary("quark")` in the spike until that `.so` is in the APK.
