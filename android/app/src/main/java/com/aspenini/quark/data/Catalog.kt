package com.aspenini.quark.data

import com.aspenini.quark.QuarkNative
import org.json.JSONArray
import org.json.JSONObject

/** Loaded from Rust `quark-gui` catalog. Do not invent extra formats. */
object Catalog {
    lateinit var AUDIO_FORMATS: List<String>
        private set
    lateinit var VIDEO_FORMATS: List<String>
        private set
    lateinit var SPACES: List<String>
        private set
    lateinit var THEMES: List<String>
        private set
    lateinit var ERR_EMPTY_QUEUE: String
        private set
    lateinit var ERR_EMPTY_OUTPUT: String
        private set
    lateinit var ERR_EMPTY_DOWNLOAD_DIR: String
        private set

    fun load() {
        val o = JSONObject(QuarkNative.catalog())
        AUDIO_FORMATS = o.getJSONArray("audio").toStringList()
        VIDEO_FORMATS = o.getJSONArray("video").toStringList()
        SPACES = o.getJSONArray("spaces").toStringList()
        THEMES = o.getJSONArray("themes").toStringList()
        ERR_EMPTY_QUEUE = o.getString("err_empty_queue")
        ERR_EMPTY_OUTPUT = o.getString("err_empty_output")
        ERR_EMPTY_DOWNLOAD_DIR = o.getString("err_empty_download_dir")
    }

    fun formatsFor(audio: Boolean) = if (audio) AUDIO_FORMATS else VIDEO_FORMATS
}

fun JSONArray.toStringList(): List<String> = List(length()) { getString(it) }
