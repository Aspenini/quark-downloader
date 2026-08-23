package com.aspenini.quark.data

import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.URL

data class AppUpdate(
    val version: String,
    val apkUrl: String,
    val htmlUrl: String,
)

object UpdateCheck {
    const val REPO = "Aspenini/quark-downloader"

    fun latest(installed: String): AppUpdate? {
        val conn =
            (URL("https://api.github.com/repos/$REPO/releases/latest").openConnection() as HttpURLConnection)
        conn.connectTimeout = 15000
        conn.readTimeout = 15000
        conn.setRequestProperty("Accept", "application/vnd.github+json")
        conn.setRequestProperty("User-Agent", "quark-downloader/$installed")
        conn.inputStream.bufferedReader().use { reader ->
            val body = JSONObject(reader.readText())
            val tag = body.optString("tag_name").trimStart('v')
            if (tag.isEmpty() || compare(installed, tag) >= 0) return null
            val assets = body.optJSONArray("assets")
            var apk: String? = null
            if (assets != null) {
                for (i in 0 until assets.length()) {
                    val name = assets.getJSONObject(i).optString("name")
                    val url = assets.getJSONObject(i).optString("browser_download_url")
                    if (name.endsWith(".apk") && name.contains("android")) {
                        apk = url
                        if (name.contains("arm64")) break
                    }
                }
            }
            val html = body.optString("html_url").ifEmpty {
                "https://github.com/$REPO/releases/latest"
            }
            return AppUpdate(
                version = tag,
                apkUrl = apk ?: html,
                htmlUrl = html,
            )
        }
    }

    fun compare(a: String, b: String): Int {
        val asv = a.trimStart('v').split('.', '-', '_').mapNotNull { it.toIntOrNull() }
        val bsv = b.trimStart('v').split('.', '-', '_').mapNotNull { it.toIntOrNull() }
        val n = maxOf(asv.size, bsv.size)
        for (i in 0 until n) {
            val av = asv.getOrElse(i) { 0 }
            val bv = bsv.getOrElse(i) { 0 }
            if (av != bv) return av.compareTo(bv)
        }
        return 0
    }
}
