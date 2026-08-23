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
    }

    companion object {
        private const val TAG = "QuarkApp"
    }
}
