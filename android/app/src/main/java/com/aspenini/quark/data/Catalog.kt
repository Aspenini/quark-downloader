package com.aspenini.quark.data

/** Mirrors crates/quark-gui/src/catalog.rs. Do not invent extra formats. */
object Catalog {
    val AUDIO_FORMATS = listOf("original", "mp3", "m4a", "flac", "wav", "opus", "vorbis")
    val VIDEO_FORMATS = listOf("original", "mp4", "mkv", "webm")
    val SPACES = listOf("keep", "underscore", "dash", "remove")
    val THEMES = listOf("system", "light", "dark")

    const val ERR_EMPTY_QUEUE = "Please enter at least one video or playlist URL."
    const val ERR_EMPTY_OUTPUT = "Please choose an output folder."
    const val ERR_EMPTY_DOWNLOAD_DIR = "Please choose a default download folder."

    fun formatsFor(audio: Boolean) = if (audio) AUDIO_FORMATS else VIDEO_FORMATS
}
