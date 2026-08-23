package com.aspenini.quark

/**
 * JNI to `libquark.so` ([crates/quark-android]). The spike APK does not load
 * this yet; the Compose frontend will once cargo-ndk ships the cdylib.
 */
object QuarkNative {
    @JvmStatic
    external fun setPaths(configDir: String)

    @JvmStatic
    external fun setJsRuntime(spec: String)

    @JvmStatic
    external fun guiScript(input: String): String

    @JvmStatic
    external fun buildYtDlpArgs(
        url: String,
        media: String,
        format: String,
        outputDir: String,
        settingsJson: String,
        ffmpegLocation: String,
        jsRuntime: String,
    ): String

    @JvmStatic
    external fun parseProgress(line: String): String

    @JvmStatic
    external fun sanitizeFilename(name: String, asciiOnly: Boolean, spaces: String): String
}
