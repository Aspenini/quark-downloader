# Android licensing and corresponding source

The Quark Downloader Android application is distributed under the GNU General
Public License, version 3 only. The complete license is in `LICENSE` and is also
packaged inside the APK under `assets/licenses/`.

The Android application source is this repository's `android/` directory and
the `crates/quark-android` JNI bridge. Shared Rust crates remain available under
their declared MIT licenses when used separately. When combined into the
Android APK, the complete work is distributed under GPL-3.0-only.

The APK includes or links the following significant third-party components:

- youtubedl-android 0.18.1 — GPL-3.0; source:
  <https://github.com/yausername/youtubedl-android/tree/0.18.1>
- yt-dlp — Unlicense; source: <https://github.com/yt-dlp/yt-dlp>
- FFmpeg as packaged by youtubedl-android — GPL-compatible build; build source
  and instructions are in the youtubedl-android repository above, with FFmpeg
  source at <https://ffmpeg.org/download.html>
- CPython — Python Software Foundation License; source:
  <https://github.com/python/cpython>
- QuickJS — MIT; source: <https://bellard.org/quickjs/>
- AndroidX, Jetpack Compose, Kotlin, and kotlinx.coroutines — Apache-2.0; source:
  <https://android.googlesource.com/platform/frameworks/support/> and
  <https://github.com/JetBrains/kotlin>
- Rust dependencies are enumerated exactly in the repository's `Cargo.lock`;
  their source and license metadata are available through crates.io.

The Apache License 2.0 text used by the Android framework dependencies is in
`APACHE-2.0`. The repository's root `LICENSE` contains the MIT license used by
the standalone desktop application and shared Rust crates.

To obtain the complete corresponding source for a released APK, check out the
Git tag matching the APK version, for example `v1.0.0`. The build instructions
are in `android/README.md`; they recreate the JNI bridge and APK from source.
