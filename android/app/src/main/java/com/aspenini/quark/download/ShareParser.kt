package com.aspenini.quark.download

import android.content.Intent

object ShareParser {
    private val URL = Regex("https?://[^\\s<>\"']+", RegexOption.IGNORE_CASE)

    fun urlsFrom(intent: Intent?): List<String> {
        if (intent == null) return emptyList()
        val chunks = mutableListOf<String>()
        intent.getStringExtra(Intent.EXTRA_TEXT)?.let(chunks::add)
        intent.getStringExtra(Intent.EXTRA_PROCESS_TEXT)?.let(chunks::add)
        intent.dataString?.let(chunks::add)
        return chunks.flatMap { extract(it) }.distinct()
    }

    fun extract(text: String): List<String> {
        val trimmed = text.trim()
        if (trimmed.startsWith("http://") || trimmed.startsWith("https://")) {
            return listOf(trimmed.split(Regex("\\s+")).first())
        }
        return URL.findAll(text).map { it.value.trimEnd('.', ',', ')', ']') }.toList()
    }
}
