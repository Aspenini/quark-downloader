package com.aspenini.quark

object QuarkNative {
    init {
        System.loadLibrary("quark")
    }

    @JvmStatic external fun setPaths(configDir: String)

    @JvmStatic external fun setJsRuntime(spec: String)

    @JvmStatic external fun catalog(): String

    @JvmStatic external fun sessionStart(defaultDir: String, settingsJson: String): String

    @JvmStatic external fun sessionDispatch(eventJson: String): String

    @JvmStatic external fun guiScript(input: String): String

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

    @JvmStatic external fun parseProgress(line: String): String

    @JvmStatic external fun isPlaylistUrl(url: String): Boolean

    @JvmStatic
    external fun sanitizeFilename(name: String, asciiOnly: Boolean, spaces: String): String

    @JvmStatic
    external fun sanitizeComponent(name: String, asciiOnly: Boolean, spaces: String): String
}
