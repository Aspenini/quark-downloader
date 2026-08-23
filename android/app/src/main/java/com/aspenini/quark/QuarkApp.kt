package com.aspenini.quark

import android.app.Application
import android.util.Log
import com.yausername.ffmpeg.FFmpeg
import com.yausername.youtubedl_android.YoutubeDL

class QuarkApp : Application() {
    override fun onCreate() {
        super.onCreate()
        try {
            YoutubeDL.init(this)
            FFmpeg.init(this)
        } catch (e: Exception) {
            Log.e(TAG, "youtubedl-android init failed", e)
        }
        com.aspenini.quark.download.YtDlpSupport.pinProcessTemp(this)
        QuarkNative.setPaths(filesDir.absolutePath)
        com.aspenini.quark.data.Catalog.load()
        Thread({
            runCatching {
                com.aspenini.quark.download.YtDlpSupport.ensureUpdated(this)
            }.onFailure { Log.w(TAG, "yt-dlp auto-update failed", it) }
        }, "ytdlp-update").start()
    }

    companion object {
        private const val TAG = "QuarkApp"
    }
}
