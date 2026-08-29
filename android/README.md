# Android

Jetpack Compose host for Quark Downloader. yt-dlp runs through
[youtubedl-android](https://github.com/yausername/youtubedl-android) 0.18.1
(embedded CPython + ffmpeg + QuickJS). **The APK is GPL-3.0** because of that
library; desktop crates stay MIT.

The Android application and JNI bridge are distributed under GPL-3.0-only.
See [`LICENSE`](LICENSE) and [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md).
Both license files are packaged into the APK under `assets/licenses/`. The Git
tag matching an APK is its complete corresponding source; preserve release tags
and all build instructions when distributing APKs.

Not published on Google Play. Install the APK from
[GitHub Releases](https://github.com/Aspenini/quark-downloader/releases)
or build it locally.

Share a link to Quark (or open a YouTube URL with it) to enqueue. Finished files
go to the system **Downloads** folder. Settings can check GitHub for a newer
APK and offer to download it.

## Build

Need JDK 17, Android SDK, NDK 26+, Python 3 (16 KB ELF alignment), and Rust
targets `aarch64-linux-android` + `x86_64-linux-android`.

```bash
just android-debug      # unsigned debug APK
just android-run        # debug APK + emulator install
just android-release    # signed release APK -> dist/quark-downloader-VERSION-android.apk
```

`just android-run` starts AVD **Quark** (android-35 google_apis x86_64) if needed.
Override with `ANDROID_AVD`. Native libs are linked and post-processed for
**16 KB page-size** devices.

## Release keystore

`just android-release` uses `android/keystore.properties` when present. Without
that file, it looks for `$HOME/quark-release.jks` (including
`%USERPROFILE%\quark-release.jks` on Windows), prompts securely for the store
password, and assumes alias `quark` with the same key password. The release
script verifies the APK certificate and 16 KB ZIP alignment before placing the
artifact in `dist/`.
Create a keystore **once**, keep it off git, and back it up.

```bash
keytool -genkeypair -v \
  -keystore "$HOME/quark-release.jks" \
  -alias quark \
  -keyalg RSA -keysize 2048 -validity 10000
```

On Windows PowerShell:

```powershell
keytool -genkeypair -v `
  -keystore "$env:USERPROFILE\quark-release.jks" `
  -alias quark `
  -keyalg RSA -keysize 2048 -validity 10000
```

For a nonstandard location or separate key password, create the gitignored
`android/keystore.properties`:

```
storeFile=/absolute/path/to/quark-release.jks
storePassword=...
keyAlias=quark
keyPassword=...
```

On Windows use a doubled-backslash path, e.g. `C:\\Users\\you\\quark-release.jks`.
Alternatively, set `QUARK_ANDROID_STORE_FILE`,
`QUARK_ANDROID_STORE_PASSWORD`, `QUARK_ANDROID_KEY_ALIAS`, and
`QUARK_ANDROID_KEY_PASSWORD` in the release process environment.

Name GitHub release assets `quark-downloader-VERSION-android.apk` so in-app
update can find them.

## Shared engine

The APK loads `libquark.so` (`crates/quark-android`). Catalog, session reducer,
yt-dlp argv, playlist detection, filename sanitize, and progress parse all live
in Rust. Gradle `preBuild` cross-compiles it via
`scripts/windows/build-android-jni.ps1` or `scripts/unix/build-android-jni.sh`.

```bash
cargo test -p quark-android
```

Legacy JNI packaging (`useLegacyPackaging`) is required so Python and ffmpeg
are executable from `nativeLibraryDir`.
