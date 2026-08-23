package com.aspenini.quark.download

import android.net.Uri
import com.aspenini.quark.data.QuarkSettings
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update

data class DownloadJob(
    val urls: List<String>,
    val audio: Boolean,
    val format: String,
    val outputDir: String,
    val settings: QuarkSettings,
)

data class DownloadUiState(
    val running: Boolean = false,
    val queueIndex: Int = 0,
    val queueTotal: Int = 0,
    val percent: Float = 0f,
    val status: String = "",
    val error: String? = null,
    val saved: List<Uri> = emptyList(),
)

object DownloadSession {
    const val PROCESS_ID = "quark-dl"

    @Volatile
    var pending: DownloadJob? = null

    private val _state = MutableStateFlow(DownloadUiState())
    val state: StateFlow<DownloadUiState> = _state.asStateFlow()

    @Volatile
    var cancelRequested: Boolean = false

    fun resetForStart(total: Int) {
        cancelRequested = false
        _state.value =
            DownloadUiState(
                running = true,
                queueIndex = 0,
                queueTotal = total,
                percent = 0f,
                status = "Starting…",
            )
    }

    fun progress(index: Int, total: Int, percent: Float, line: String) {
        _state.update {
            it.copy(
                running = true,
                queueIndex = index,
                queueTotal = total,
                percent = percent.coerceIn(0f, 100f),
                status = line.trim().ifEmpty { it.status },
            )
        }
    }

    fun finished(saved: List<Uri>, error: String?) {
        _state.update {
            it.copy(
                running = false,
                percent = if (error == null) 100f else it.percent,
                status = error ?: if (saved.isEmpty()) "Done." else "Saved ${saved.size} file(s).",
                error = error,
                saved = saved,
            )
        }
    }
}
