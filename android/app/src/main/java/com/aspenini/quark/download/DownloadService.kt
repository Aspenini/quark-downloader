package com.aspenini.quark.download

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import androidx.core.app.NotificationCompat
import androidx.core.app.ServiceCompat
import com.aspenini.quark.MainActivity
import com.aspenini.quark.QuarkNative
import com.aspenini.quark.R
import com.yausername.youtubedl_android.YoutubeDL
import com.yausername.youtubedl_android.YoutubeDLRequest
import org.json.JSONArray
import org.json.JSONObject
import java.io.File

class DownloadService : Service() {
    @Volatile
    private var worker: Thread? = null

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent?.action == ACTION_CANCEL) {
            DownloadSession.cancelRequested = true
            YoutubeDL.destroyProcessById(DownloadSession.PROCESS_ID)
            return START_NOT_STICKY
        }
        val job = DownloadSession.pending ?: return START_NOT_STICKY
        startInForeground(job.urls.size)
        if (worker?.isAlive == true) {
            return START_NOT_STICKY
        }
        worker =
            Thread {
                try {
                    runJob(job)
                } finally {
                    stopForeground(STOP_FOREGROUND_DETACH)
                    stopSelf()
                }
            }.also { it.start() }
        return START_NOT_STICKY
    }

    private fun runJob(job: DownloadJob) {
        DownloadSession.resetForStart(job.urls.size)
        val saved = mutableListOf<android.net.Uri>()
        val failures = mutableListOf<String>()
        val publicRoot = File(job.outputDir)
        val workRoot = File(cacheDir, "quark-dl").also { it.mkdirs() }

        for ((index, url) in job.urls.withIndex()) {
            if (DownloadSession.cancelRequested) {
                failures += "Cancelled"
                break
            }
            val n = index + 1
            notifyProgress(n, job.urls.size, 0, "URL $n of ${job.urls.size}")
            DownloadSession.progress(n, job.urls.size, 0f, "URL $n of ${job.urls.size}")
            DownloadLog.append(this, job.settings.downloadLogs, "==> URL $n of ${job.urls.size}: $url")
            val workDir = File(workRoot, "item-$n").also {
                it.deleteRecursively()
                it.mkdirs()
            }
            val playlist = QuarkNative.isPlaylistUrl(url)
            var subdir: String? = null
            if (playlist && job.settings.playlistFolders) {
                probePlaylistTitle(url)?.let { title ->
                    subdir =
                        QuarkNative.sanitizeComponent(
                            title,
                            job.settings.sanitizeFilenames,
                            job.settings.filenameSpaces,
                        )
                }
            }
            try {
                val request = YoutubeDLRequest(url)
                applyRustArgs(
                    request,
                    QuarkNative.buildYtDlpArgs(
                        url,
                        if (job.audio) "audio" else "video",
                        job.format,
                        workDir.absolutePath,
                        job.settings.toJson(),
                        "",
                        "",
                    ),
                )
                YoutubeDL.execute(request, DownloadSession.PROCESS_ID) { percent, _, line ->
                    val status = statusFromRust(line)
                    DownloadSession.progress(n, job.urls.size, percent, status)
                    notifyProgress(n, job.urls.size, percent.toInt(), status)
                    DownloadLog.append(this, job.settings.downloadLogs, line)
                }
                saved +=
                    MediaPublisher.publishTree(this, workDir, publicRoot, subdir)
            } catch (e: Exception) {
                if (DownloadSession.cancelRequested) {
                    failures += "Cancelled"
                    break
                }
                failures += url
                DownloadLog.append(this, job.settings.downloadLogs, "ERROR $url: ${e.message}")
            } finally {
                workDir.deleteRecursively()
            }
        }

        val error =
            when {
                DownloadSession.cancelRequested -> "Cancelled."
                failures.isEmpty() -> null
                else -> "Failed ${failures.size} of ${job.urls.size}."
            }
        DownloadSession.finished(saved, error)
        DownloadLog.append(
            this,
            job.settings.downloadLogs,
            error ?: "Saved ${saved.size} file(s).",
        )
        notifyDone(saved.size, error)
        if (error == null && job.settings.openOutputDir) {
            val open = Intent(android.app.DownloadManager.ACTION_VIEW_DOWNLOADS)
            open.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            runCatching { startActivity(open) }
        }
    }

    private fun probePlaylistTitle(url: String): String? {
        return try {
            val request = YoutubeDLRequest(url)
            request.addOption("--flat-playlist")
            request.addOption("-I", "1:1")
            request.addOption("-J")
            request.addOption("--no-warnings")
            request.addOption("--no-color")
            val out = YoutubeDL.execute(request, null, null).out
            val json = JSONObject(out)
            if (json.optString("_type") != "playlist") return null
            json.optString("title").trim().takeIf { it.isNotEmpty() }
        } catch (_: Exception) {
            null
        }
    }

    private fun startInForeground(total: Int) {
        ensureChannel()
        val notification = notification(0, total, 0, "Starting…")
        if (Build.VERSION.SDK_INT >= 29) {
            ServiceCompat.startForeground(
                this,
                NOTIFY_ID,
                notification,
                ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC,
            )
        } else {
            startForeground(NOTIFY_ID, notification)
        }
    }

    private fun notifyProgress(index: Int, total: Int, percent: Int, line: String) {
        val nm = getSystemService(NotificationManager::class.java)
        nm.notify(NOTIFY_ID, notification(index, total, percent, line))
    }

    private fun notifyDone(saved: Int, error: String?) {
        val nm = getSystemService(NotificationManager::class.java)
        val text = error ?: "Saved $saved file(s) to Downloads"
        val notification =
            NotificationCompat.Builder(this, CHANNEL)
                .setSmallIcon(android.R.drawable.stat_sys_download_done)
                .setContentTitle(getString(R.string.app_name))
                .setContentText(text)
                .setContentIntent(openApp())
                .setAutoCancel(true)
                .build()
        nm.notify(NOTIFY_ID, notification)
    }

    private fun notification(index: Int, total: Int, percent: Int, line: String): Notification {
        val title =
            if (total > 1 && index > 0) {
                "URL $index of $total"
            } else {
                getString(R.string.app_name)
            }
        val cancel =
            PendingIntent.getService(
                this,
                1,
                Intent(this, DownloadService::class.java).setAction(ACTION_CANCEL),
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
            )
        return NotificationCompat.Builder(this, CHANNEL)
            .setSmallIcon(android.R.drawable.stat_sys_download)
            .setContentTitle(title)
            .setContentText(line.take(80))
            .setProgress(100, percent.coerceIn(0, 100), percent <= 0)
            .setContentIntent(openApp())
            .addAction(0, "Cancel", cancel)
            .setOngoing(true)
            .setOnlyAlertOnce(true)
            .build()
    }

    private fun openApp(): PendingIntent {
        val launch = Intent(this, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP)
        return PendingIntent.getActivity(
            this,
            0,
            launch,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
    }

    private fun ensureChannel() {
        if (Build.VERSION.SDK_INT < 26) return
        val nm = getSystemService(NotificationManager::class.java)
        nm.createNotificationChannel(
            NotificationChannel(CHANNEL, "Downloads", NotificationManager.IMPORTANCE_LOW),
        )
    }

    private fun statusFromRust(line: String): String {
        val parsed = QuarkNative.parseProgress(line)
        if (parsed == "null" || parsed.isBlank()) return line
        return runCatching {
            JSONObject(parsed).optString("status").ifEmpty { line }
        }.getOrDefault(line)
    }

    private fun applyRustArgs(request: YoutubeDLRequest, json: String) {
        val arr = JSONArray(json)
        for (i in 0 until arr.length()) {
            val opt = arr.getJSONObject(i)
            val name = opt.getString("n")
            if (opt.has("v")) {
                request.addOption(name, opt.getString("v"))
            } else {
                request.addOption(name)
            }
        }
    }

    companion object {
        const val ACTION_CANCEL = "com.aspenini.quark.CANCEL_DOWNLOAD"
        private const val CHANNEL = "downloads"
        private const val NOTIFY_ID = 42
    }
}
