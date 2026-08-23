package com.aspenini.quark

import android.app.Application
import android.content.Intent
import android.os.Build
import androidx.core.content.ContextCompat
import androidx.lifecycle.AndroidViewModel
import com.aspenini.quark.data.Catalog
import com.aspenini.quark.data.QuarkSettings
import com.aspenini.quark.data.SettingsStore
import com.aspenini.quark.download.DownloadJob
import com.aspenini.quark.download.DownloadService
import com.aspenini.quark.download.DownloadSession
import com.aspenini.quark.download.ShareParser
import com.yausername.youtubedl_android.YoutubeDL
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update

data class UiSnapshot(
    val urlField: String = "",
    val queue: List<String> = emptyList(),
    val audio: Boolean = false,
    val format: String = "original",
    val output: String = QuarkSettings.defaults().downloadDir,
    val settings: QuarkSettings = QuarkSettings.defaults(),
    val draft: QuarkSettings = QuarkSettings.defaults(),
    val showSettings: Boolean = false,
    val snackbar: String? = null,
    val ytDlpVersion: String? = null,
)

class QuarkViewModel(app: Application) : AndroidViewModel(app) {
    private val store = SettingsStore(app)
    private val _ui: MutableStateFlow<UiSnapshot>

    init {
        val settings = store.load()
        _ui =
            MutableStateFlow(
                UiSnapshot(
                    output = settings.downloadDir,
                    settings = settings,
                    draft = settings,
                ),
            )
    }

    val ui: StateFlow<UiSnapshot> = _ui.asStateFlow()
    val download = DownloadSession.state

    fun formats(): List<String> = Catalog.formatsFor(_ui.value.audio)

    fun setUrlField(value: String) {
        _ui.update { it.copy(urlField = value) }
    }

    fun addUrl() {
        val url = _ui.value.urlField.trim()
        if (url.isEmpty()) return
        _ui.update { state ->
            val queue = if (state.queue.contains(url)) state.queue else state.queue + url
            state.copy(urlField = "", queue = queue)
        }
    }

    fun paste(text: String) {
        val urls = ShareParser.extract(text).ifEmpty {
            text.split(Regex("\\s+")).map { it.trim() }.filter { it.isNotEmpty() }
        }
        if (urls.isEmpty()) return
        _ui.update { state ->
            val queue = state.queue.toMutableList()
            for (url in urls) {
                if (url !in queue) queue += url
            }
            state.copy(urlField = "", queue = queue)
        }
    }

    fun ingestIntent(intent: Intent?) {
        val urls = ShareParser.urlsFrom(intent)
        if (urls.isEmpty()) return
        _ui.update { state ->
            val queue = state.queue.toMutableList()
            for (url in urls) {
                if (url !in queue) queue += url
            }
            state.copy(
                showSettings = false,
                snackbar = if (urls.size == 1) "Added to queue" else "Added ${urls.size} URLs",
                queue = queue,
            )
        }
    }

    fun removeAt(index: Int) {
        _ui.update { state ->
            if (index !in state.queue.indices) return@update state
            state.copy(queue = state.queue.toMutableList().also { it.removeAt(index) })
        }
    }

    fun setAudio(audio: Boolean) {
        _ui.update { it.copy(audio = audio, format = "original") }
    }

    fun setFormat(format: String) {
        if (format !in Catalog.formatsFor(_ui.value.audio)) return
        _ui.update { it.copy(format = format) }
    }

    fun openSettings() {
        _ui.update { it.copy(showSettings = true, draft = it.settings) }
    }

    fun closeSettings() {
        _ui.update { it.copy(showSettings = false, draft = it.settings) }
    }

    fun updateDraft(draft: QuarkSettings) {
        _ui.update { it.copy(draft = draft) }
    }

    fun resetDraft() {
        _ui.update { it.copy(draft = QuarkSettings.defaults()) }
    }

    fun saveSettings() {
        val draft = _ui.value.draft
        if (draft.downloadDir.trim().isEmpty()) {
            snack(Catalog.ERR_EMPTY_DOWNLOAD_DIR)
            return
        }
        store.save(draft)
        _ui.update {
            it.copy(
                settings = draft,
                output = draft.downloadDir,
                showSettings = false,
            )
        }
    }

    fun download() {
        addUrl()
        val state = _ui.value
        if (state.queue.isEmpty()) {
            snack(Catalog.ERR_EMPTY_QUEUE)
            return
        }
        if (state.output.trim().isEmpty()) {
            snack(Catalog.ERR_EMPTY_OUTPUT)
            return
        }
        if (DownloadSession.state.value.running) {
            snack("A download is already running.")
            return
        }
        DownloadSession.pending =
            DownloadJob(
                urls = state.queue,
                audio = state.audio,
                format = state.format,
                outputDir = state.output.trim(),
                settings = state.settings,
            )
        val intent = Intent(getApplication(), DownloadService::class.java)
        ContextCompat.startForegroundService(getApplication(), intent)
    }

    fun cancelDownload() {
        DownloadSession.cancelRequested = true
        YoutubeDL.destroyProcessById(DownloadSession.PROCESS_ID)
        val intent =
            Intent(getApplication(), DownloadService::class.java).setAction(DownloadService.ACTION_CANCEL)
        getApplication<Application>().startService(intent)
    }

    fun consumeSnackbar() {
        _ui.update { it.copy(snackbar = null) }
    }

    fun snack(message: String) {
        _ui.update { it.copy(snackbar = message) }
    }

    fun refreshVersion() {
        val ctx = getApplication<Application>()
        val version = YoutubeDL.versionName(ctx) ?: YoutubeDL.version(ctx)
        _ui.update { it.copy(ytDlpVersion = version) }
    }

    fun updateYtDlp(): String {
        val ctx = getApplication<Application>()
        val status = YoutubeDL.updateYoutubeDL(ctx, YoutubeDL.UpdateChannel.STABLE)
        val version = YoutubeDL.versionName(ctx) ?: YoutubeDL.version(ctx)
        _ui.update { it.copy(ytDlpVersion = version) }
        return "yt-dlp $status ($version)"
    }

    fun needsStoragePermission(): Boolean = Build.VERSION.SDK_INT < 29
}
