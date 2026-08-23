package com.aspenini.quark.download

import android.content.ContentValues
import android.content.Context
import android.media.MediaScannerConnection
import android.net.Uri
import android.os.Build
import android.os.Environment
import android.provider.MediaStore
import android.webkit.MimeTypeMap
import java.io.File

object MediaPublisher {
    fun publishTree(
        context: Context,
        workDir: File,
        publicRoot: File,
        relativeSubdir: String?,
    ): List<Uri> {
        val files =
            workDir.walkTopDown()
                .filter { it.isFile }
                .filter { !it.name.endsWith(".part") && !it.name.endsWith(".ytdl") }
                .toList()
        val published = mutableListOf<Uri>()
        for (file in files) {
            publishOne(context, file, publicRoot, relativeSubdir)?.let(published::add)
        }
        return published
    }

    private fun publishOne(
        context: Context,
        file: File,
        publicRoot: File,
        relativeSubdir: String?,
    ): Uri? {
        val destDir =
            if (relativeSubdir.isNullOrBlank()) {
                publicRoot
            } else {
                File(publicRoot, relativeSubdir)
            }
        if (canWriteDirect(destDir)) {
            destDir.mkdirs()
            val dest = uniqueFile(destDir, file.name)
            file.copyTo(dest, overwrite = false)
            MediaScannerConnection.scanFile(context, arrayOf(dest.absolutePath), null, null)
            file.delete()
            return Uri.fromFile(dest)
        }
        if (Build.VERSION.SDK_INT >= 29) {
            return publishMediaStore(context, file, relativeSubdir)
        }
        return null
    }

    fun canWriteDirect(dir: File): Boolean {
        return try {
            dir.mkdirs()
            val probe = File(dir, ".quark-write-test-${System.nanoTime()}")
            probe.writeText("ok")
            probe.delete()
            true
        } catch (_: Exception) {
            false
        }
    }

    private fun publishMediaStore(context: Context, file: File, relativeSubdir: String?): Uri? {
        val rel =
            if (relativeSubdir.isNullOrBlank()) {
                Environment.DIRECTORY_DOWNLOADS
            } else {
                Environment.DIRECTORY_DOWNLOADS + "/" + relativeSubdir
            }
        val mime = mimeOf(file.name)
        val values =
            ContentValues().apply {
                put(MediaStore.MediaColumns.DISPLAY_NAME, file.name)
                put(MediaStore.MediaColumns.MIME_TYPE, mime)
                put(MediaStore.MediaColumns.RELATIVE_PATH, rel)
                put(MediaStore.MediaColumns.IS_PENDING, 1)
            }
        val collection = MediaStore.Downloads.getContentUri(MediaStore.VOLUME_EXTERNAL_PRIMARY)
        val uri = context.contentResolver.insert(collection, values) ?: return null
        try {
            context.contentResolver.openOutputStream(uri)?.use { out ->
                file.inputStream().use { it.copyTo(out) }
            } ?: run {
                context.contentResolver.delete(uri, null, null)
                return null
            }
            values.clear()
            values.put(MediaStore.MediaColumns.IS_PENDING, 0)
            context.contentResolver.update(uri, values, null, null)
            file.delete()
            return uri
        } catch (_: Exception) {
            context.contentResolver.delete(uri, null, null)
            return null
        }
    }

    private fun uniqueFile(dir: File, name: String): File {
        val dest = File(dir, name)
        if (!dest.exists()) return dest
        val dot = name.lastIndexOf('.')
        val stem = if (dot > 0) name.substring(0, dot) else name
        val ext = if (dot > 0) name.substring(dot) else ""
        for (n in 2..99) {
            val candidate = File(dir, "$stem ($n)$ext")
            if (!candidate.exists()) return candidate
        }
        return File(dir, "$stem-${System.currentTimeMillis()}$ext")
    }

    private fun mimeOf(name: String): String {
        val ext = name.substringAfterLast('.', "").lowercase()
        return MimeTypeMap.getSingleton().getMimeTypeFromExtension(ext)
            ?: when (ext) {
                "mp3" -> "audio/mpeg"
                "m4a" -> "audio/mp4"
                "opus", "ogg" -> "audio/ogg"
                "flac" -> "audio/flac"
                "wav" -> "audio/wav"
                "mp4" -> "video/mp4"
                "mkv" -> "video/x-matroska"
                "webm" -> "video/webm"
                else -> "application/octet-stream"
            }
    }
}
