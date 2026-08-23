package com.aspenini.quark.download

import android.content.Context
import java.io.File
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

object DownloadLog {
    const val KEEP = 10

    fun dir(context: Context): File = File(context.filesDir, "logs").also { it.mkdirs() }

    fun append(context: Context, enabled: Boolean, line: String) {
        if (!enabled) return
        val day = SimpleDateFormat("yyyyMMdd", Locale.US).format(Date())
        val file = File(dir(context), "$day.log")
        file.appendText(line.trimEnd() + "\n")
        prune(dir(context))
    }

    fun latest(context: Context): File? =
        dir(context)
            .listFiles { f -> f.isFile && f.extension == "log" }
            ?.maxByOrNull { it.lastModified() }

    private fun prune(dir: File) {
        val files =
            dir.listFiles { f -> f.isFile && f.extension == "log" }
                ?.sortedBy { it.lastModified() }
                ?: return
        val excess = files.size - KEEP
        if (excess > 0) {
            files.take(excess).forEach { it.delete() }
        }
    }
}
