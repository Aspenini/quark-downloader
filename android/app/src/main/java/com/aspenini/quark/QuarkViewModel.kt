package com.aspenini.quark

import android.app.Application
import android.content.Intent
import androidx.core.content.ContextCompat
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.aspenini.quark.data.AppUpdate
import com.aspenini.quark.data.Catalog
import com.aspenini.quark.data.QuarkSettings
import com.aspenini.quark.data.SettingsStore
import com.aspenini.quark.data.UpdateCheck
import com.aspenini.quark.data.toStringList
import com.aspenini.quark.download.DownloadJob
import com.aspenini.quark.download.DownloadService
import com.aspenini.quark.download.DownloadSession
import com.aspenini.quark.download.ShareParser
import com.yausername.youtubedl_android.YoutubeDL
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONObject
import com.aspenini.quark.BuildConfig

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
    val formats: List<String> = emptyList(),
    val appUpdate: AppUpdate? = null,
)

class QuarkViewModel(app: Application) : AndroidViewModel(app) {
    private val store = SettingsStore(app)
    private val _ui = MutableStateFlow(UiSnapshot())

    val ui: StateFlow<UiSnapshot> = _ui.asStateFlow()
    val download = DownloadSession.state

    init {
        val settings = store.load()
        val started =
            JSONObject(
                QuarkNative.sessionStart(settings.downloadDir, settings.toJson()),
            )
        applyDispatch(started)
        viewModelScope.launch { runCatching { checkAppUpdate() } }
    }

    fun formats(): List<String> = _ui.value.formats.ifEmpty { Catalog.formatsFor(_ui.value.audio) }

    fun setUrlField(value: String) {
        dispatch(JSONObject().put("set_url_field", value))
    }

    fun addUrl() {
        val url = _ui.value.urlField
        dispatch(JSONObject().put("add_url", url))
    }

    fun paste(text: String) {
        dispatch(JSONObject().put("paste", text))
    }

    fun ingestIntent(intent: Intent?) {
        val urls = ShareParser.urlsFrom(intent)
        if (urls.isEmpty()) return
        dispatch(JSONObject().put("paste", urls.joinToString(" ")))
        val msg = if (urls.size == 1) "Added to queue" else "Added ${urls.size} URLs"
        _ui.update { it.copy(snackbar = msg) }
    }

    fun removeAt(index: Int) {
        dispatch(JSONObject().put("select", index))
        dispatch(JSONObject().put("remove_selected", true))
    }

    fun setAudio(audio: Boolean) {
        dispatch(JSONObject().put("set_media", if (audio) "audio" else "video"))
    }

    fun setFormat(format: String) {
        dispatch(JSONObject().put("set_format", format))
    }

    fun openSettings() {
        dispatch(JSONObject().put("open_settings", true))
    }

    fun closeSettings() {
        dispatch(JSONObject().put("close_settings", true))
    }

    fun updateDraft(draft: QuarkSettings) {
        dispatch(JSONObject().put("set_setting", JSONObject(draft.toJson())))
    }

    fun resetDraft() {
        dispatch(JSONObject().put("reset_settings", true))
    }

    fun saveSettings() {
        dispatch(JSONObject().put("save_settings", true))
        if (!_ui.value.showSettings) {
            store.save(_ui.value.settings)
        }
    }

    fun download() {
        dispatch(JSONObject().put("download", true))
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

    suspend fun checkAppUpdate(): AppUpdate? {
        val found =
            withContext(Dispatchers.IO) { UpdateCheck.latest(BuildConfig.VERSION_NAME) }
        _ui.update { it.copy(appUpdate = found) }
        return found
    }

    fun dismissAppUpdate() {
        _ui.update { it.copy(appUpdate = null) }
    }

    fun updateYtDlp(): String {
        val ctx = getApplication<Application>()
        val status = YoutubeDL.updateYoutubeDL(ctx, YoutubeDL.UpdateChannel.STABLE)
        val version = YoutubeDL.versionName(ctx) ?: YoutubeDL.version(ctx)
        _ui.update { it.copy(ytDlpVersion = version) }
        return "yt-dlp $status ($version)"
    }

    private fun dispatch(event: JSONObject) {
        applyDispatch(JSONObject(QuarkNative.sessionDispatch(event.toString())))
    }

    private fun applyDispatch(out: JSONObject) {
        val state = out.getJSONObject("state")
        val settings = QuarkSettings.fromJson(state.getJSONObject("settings"))
        val draft = QuarkSettings.fromJson(state.getJSONObject("draft"))
        val media = state.optString("media") == "audio"
        _ui.update { prev ->
            prev.copy(
                urlField = state.optString("url_field"),
                queue = state.getJSONArray("queue").toStringList(),
                audio = media,
                format = state.optString("format", "original"),
                output = state.optString("output", settings.downloadDir),
                settings = settings,
                draft = draft,
                showSettings = state.optString("view") == "settings",
                formats = state.optJSONArray("formats")?.toStringList() ?: Catalog.formatsFor(media),
            )
        }
        val effects = out.optJSONArray("effects") ?: return
        for (i in 0 until effects.length()) {
            val effect = effects.getJSONObject(i)
            when {
                effect.has("error") -> snack(effect.getString("error"))
                effect.has("emit") -> onEmit(effect.getJSONObject("emit"))
            }
        }
    }

    private fun onEmit(emit: JSONObject) {
        if (emit.optString("action") != "download") return
        if (DownloadSession.state.value.running) {
            snack("A download is already running.")
            return
        }
        val urls = emit.getJSONArray("urls").toStringList()
        DownloadSession.pending =
            DownloadJob(
                urls = urls,
                audio = emit.optString("media_type") == "audio",
                format = emit.optString("format"),
                outputDir = emit.optString("output_dir"),
                settings = _ui.value.settings,
            )
        val intent = Intent(getApplication(), DownloadService::class.java)
        ContextCompat.startForegroundService(getApplication(), intent)
    }
}
