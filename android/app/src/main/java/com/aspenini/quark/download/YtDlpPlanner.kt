package com.aspenini.quark.download

import com.aspenini.quark.data.QuarkSettings
import com.yausername.youtubedl_android.YoutubeDLRequest
import java.io.File

/** Argv matching crates/quark-core ytdlp::plan. ffmpeg/QuickJS are injected by youtubedl-android. */
object YtDlpPlanner {
    data class Opt(val name: String, val value: String? = null)

    fun needsFfmpeg(format: String) = format != "original"

    fun build(
        url: String,
        audio: Boolean,
        format: String,
        targetDir: File,
        settings: QuarkSettings,
        playlist: Boolean,
    ): List<Opt> {
        val template =
            if (settings.stripVideoIds) {
                "%(title)s.%(ext)s"
            } else {
                "%(title)s [%(id)s].%(ext)s"
            }
        val outtmpl = File(targetDir, template).absolutePath
        val args = mutableListOf<Opt>()
        if (playlist) {
            args += Opt("--yes-playlist")
            args += Opt("--ignore-errors")
        } else {
            args += Opt("--no-playlist")
        }
        args += Opt("-o", outtmpl)
        args += Opt("--socket-timeout", "30")
        args += Opt("--retries", "3")
        args += Opt("--fragment-retries", "3")
        if (audio) {
            args += Opt("-f", "bestaudio/best")
            if (needsFfmpeg(format)) {
                args += Opt("-x")
                args += Opt("--audio-format", format)
            }
        } else if (needsFfmpeg(format)) {
            args += Opt("-f", "bv*+ba/b")
            args += Opt("--merge-output-format", format)
            when (format) {
                "webm" -> args += Opt("--recode-video", "webm")
                "mp4" -> args += Opt("--remux-video", "mp4")
            }
        }
        args += Opt("--newline")
        args += Opt("--windows-filenames")
        if (settings.sanitizeFilenames) {
            args += Opt("--restrict-filenames")
        }
        args += Opt("--no-color")
        return args
    }

    fun apply(request: YoutubeDLRequest, opts: List<Opt>) {
        for (opt in opts) {
            val value = opt.value
            if (value == null) {
                request.addOption(opt.name)
            } else {
                request.addOption(opt.name, value)
            }
        }
    }
}
