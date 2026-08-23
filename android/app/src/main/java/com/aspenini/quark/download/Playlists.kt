package com.aspenini.quark.download

import android.net.Uri

/** Same heuristic as crates/quark-core/src/playlist.rs. */
fun isPlaylistUrl(url: String): Boolean {
    val uri = Uri.parse(url) ?: return false
    val host = (uri.host ?: "").lowercase()
    val path = (uri.path ?: "").lowercase()
    if (host == "youtu.be" || host.endsWith(".youtu.be")) {
        return false
    }
    if (path.contains("/playlist") || path.contains("/playlists/") || path.contains("/sets/")) {
        return true
    }
    val hasList = !uri.getQueryParameter("list").isNullOrEmpty() ||
        !uri.getQueryParameter("p").isNullOrEmpty()
    val hasV = !uri.getQueryParameter("v").isNullOrEmpty()
    return hasList && !hasV
}

fun sanitizeFolder(name: String): String {
    val invalid = "<>:\"/\\|?*"
    val cleaned =
        name.map { c ->
            if (c.code < 32 || c in invalid) '-' else c
        }.joinToString("")
            .trim('.', ' ')
    return cleaned.take(180).ifEmpty { "playlist" }
}
