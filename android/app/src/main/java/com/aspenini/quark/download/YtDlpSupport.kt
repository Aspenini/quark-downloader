package com.aspenini.quark.download

import android.content.Context
import android.system.Os
import android.util.Log
import com.yausername.youtubedl_android.YoutubeDL
import com.yausername.youtubedl_android.YoutubeDLRequest
import java.io.File

object YtDlpSupport {
    private const val TAG = "YtDlpSupport"

    fun tempDir(context: Context): File =
        File(context.cacheDir, "ytdlp-tmp").also { it.mkdirs() }

    fun pinProcessTemp(context: Context) {
        val tmp = tempDir(context).absolutePath
        runCatching {
            Os.setenv("TMPDIR", tmp, true)
            Os.setenv("TEMP", tmp, true)
            Os.setenv("TMP", tmp, true)
        }.onFailure { Log.w(TAG, "setenv TMPDIR failed", it) }
    }

    /** youtubedl-android ships a stale yt-dlp; YouTube 403s until this runs. */
    @Synchronized
    fun ensureUpdated(context: Context): String {
        pinProcessTemp(context)
        val status =
            YoutubeDL.updateYoutubeDL(context, YoutubeDL.UpdateChannel.STABLE)
        val version = YoutubeDL.versionName(context) ?: YoutubeDL.version(context) ?: "?"
        Log.i(TAG, "yt-dlp update $status ($version)")
        return "$status ($version)"
    }

    fun applyAndroidPaths(request: YoutubeDLRequest, context: Context) {
        val tmp = tempDir(context).absolutePath
        // Library adds --no-cache-dir unless --cache-dir is present.
        request.addOption("--cache-dir", tmp)
        request.addOption("--paths", "temp:$tmp")
        request.addOption("--no-update")
    }
}
