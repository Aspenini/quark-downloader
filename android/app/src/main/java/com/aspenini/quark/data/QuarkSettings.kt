package com.aspenini.quark.data

import android.content.Context
import android.os.Environment
import org.json.JSONObject
import java.io.File

data class QuarkSettings(
    val downloadDir: String,
    val downloadLogs: Boolean,
    val openOutputDir: Boolean,
    val guiTheme: String,
    val stripVideoIds: Boolean,
    val sanitizeFilenames: Boolean,
    val filenameSpaces: String,
    val playlistFolders: Boolean,
) {
    companion object {
        fun publicDownloads(): File =
            Environment.getExternalStoragePublicDirectory(Environment.DIRECTORY_DOWNLOADS)

        fun defaults(): QuarkSettings {
            val dir = publicDownloads().absolutePath
            return QuarkSettings(
                downloadDir = dir,
                downloadLogs = true,
                openOutputDir = false,
                guiTheme = "system",
                stripVideoIds = true,
                sanitizeFilenames = true,
                filenameSpaces = "keep",
                playlistFolders = true,
            )
        }

        fun fromJson(o: JSONObject) =
            QuarkSettings(
                downloadDir = o.optString("download_dir", defaults().downloadDir),
                downloadLogs = o.optBoolean("download_logs", true),
                openOutputDir = o.optBoolean("open_output_dir", false),
                guiTheme = o.optString("gui_theme", "system"),
                stripVideoIds = o.optBoolean("strip_video_ids", true),
                sanitizeFilenames = o.optBoolean("sanitize_filenames", true),
                filenameSpaces = o.optString("filename_spaces", "keep"),
                playlistFolders = o.optBoolean("playlist_folders", true),
            )
    }

    fun toJson(): String =
        JSONObject()
            .put("download_dir", downloadDir)
            .put("yt_dlp", "auto")
            .put("ffmpeg", "auto")
            .put("gui_download_mode", "progress")
            .put("download_logs", downloadLogs)
            .put("open_output_dir", openOutputDir)
            .put("gui_theme", guiTheme)
            .put("strip_video_ids", stripVideoIds)
            .put("sanitize_filenames", sanitizeFilenames)
            .put("filename_spaces", filenameSpaces)
            .put("playlist_folders", playlistFolders)
            .toString()
}

class SettingsStore(context: Context) {
    private val prefs = context.applicationContext.getSharedPreferences("quark", Context.MODE_PRIVATE)

    fun load(): QuarkSettings {
        val d = QuarkSettings.defaults()
        return QuarkSettings(
            downloadDir = prefs.getString("download_dir", d.downloadDir) ?: d.downloadDir,
            downloadLogs = prefs.getBoolean("download_logs", d.downloadLogs),
            openOutputDir = prefs.getBoolean("open_output_dir", d.openOutputDir),
            guiTheme = prefs.getString("gui_theme", d.guiTheme) ?: d.guiTheme,
            stripVideoIds = prefs.getBoolean("strip_video_ids", d.stripVideoIds),
            sanitizeFilenames = prefs.getBoolean("sanitize_filenames", d.sanitizeFilenames),
            filenameSpaces = prefs.getString("filename_spaces", d.filenameSpaces) ?: d.filenameSpaces,
            playlistFolders = prefs.getBoolean("playlist_folders", d.playlistFolders),
        )
    }

    fun save(s: QuarkSettings) {
        prefs.edit()
            .putString("download_dir", s.downloadDir)
            .putBoolean("download_logs", s.downloadLogs)
            .putBoolean("open_output_dir", s.openOutputDir)
            .putString("gui_theme", s.guiTheme)
            .putBoolean("strip_video_ids", s.stripVideoIds)
            .putBoolean("sanitize_filenames", s.sanitizeFilenames)
            .putString("filename_spaces", s.filenameSpaces)
            .putBoolean("playlist_folders", s.playlistFolders)
            .apply()
    }
}
