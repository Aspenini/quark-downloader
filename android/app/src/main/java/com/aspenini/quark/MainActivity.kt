package com.aspenini.quark

import android.content.Context
import android.os.Bundle
import android.os.Environment
import android.os.Handler
import android.os.Looper
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.yausername.youtubedl_android.YoutubeDL
import com.yausername.youtubedl_android.YoutubeDLRequest
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.io.File

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        val outDir =
            getExternalFilesDir(Environment.DIRECTORY_DOWNLOADS)
                ?: File(filesDir, "downloads").also { it.mkdirs() }
        val app = applicationContext
        setContent {
            MaterialTheme {
                Surface(modifier = Modifier.fillMaxSize()) {
                    SpikeScreen(appContext = app, outDir = outDir)
                }
            }
        }
    }
}

private const val PROCESS_ID = "quark-spike"
private const val DEFAULT_URL = "https://www.youtube.com/watch?v=jNQXAC9IVRw"

@Composable
private fun SpikeScreen(appContext: Context, outDir: File) {
    val scope = rememberCoroutineScope()
    val mainHandler = remember { Handler(Looper.getMainLooper()) }
    var url by remember { mutableStateOf(DEFAULT_URL) }
    var log by remember { mutableStateOf("Output: ${outDir.absolutePath}\n") }
    var busy by remember { mutableStateOf(false) }
    var progress by remember { mutableFloatStateOf(0f) }

    fun append(line: String) {
        log += line.trimEnd() + "\n"
    }

    fun runJob(label: String, block: () -> String) {
        if (busy) return
        busy = true
        progress = 0f
        append("==> $label")
        scope.launch {
            val result =
                withContext(Dispatchers.IO) {
                    runCatching(block).fold(
                        onSuccess = { it },
                        onFailure = { e -> "ERROR: ${e.message}\n${e.stackTraceToString()}" },
                    )
                }
            append(result)
            busy = false
        }
    }

    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text("Quark Android spike", style = MaterialTheme.typography.titleLarge)
        Text(
            "Kill criterion: version, YouTube download, ffmpeg remux/extract. " +
                "youtubedl-android injects QuickJS + ffmpeg.",
            style = MaterialTheme.typography.bodySmall,
        )
        OutlinedTextField(
            value = url,
            onValueChange = { url = it },
            label = { Text("URL") },
            modifier = Modifier.fillMaxWidth(),
            enabled = !busy,
            singleLine = true,
        )
        if (busy) {
            LinearProgressIndicator(
                progress = { progress.coerceIn(0f, 100f) / 100f },
                modifier = Modifier.fillMaxWidth(),
            )
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button(
                onClick = {
                    runJob("version") {
                        YoutubeDL.versionName(appContext)
                            ?: YoutubeDL.version(appContext)
                            ?: "(unknown version)"
                    }
                },
                enabled = !busy,
            ) { Text("Version") }
            Button(
                onClick = {
                    runJob("update yt-dlp") {
                        val status =
                            YoutubeDL.updateYoutubeDL(
                                appContext,
                                YoutubeDL.UpdateChannel.STABLE,
                            )
                        "update: $status\nversion: ${YoutubeDL.versionName(appContext)}"
                    }
                },
                enabled = !busy,
            ) { Text("Update") }
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button(
                onClick = {
                    runJob("download original") {
                        executeDownload(url, outDir, mp3 = false) { p, line ->
                            mainHandler.post {
                                progress = p
                                append(line)
                            }
                        }
                    }
                },
                enabled = !busy,
            ) { Text("Download") }
            Button(
                onClick = {
                    runJob("extract mp3") {
                        executeDownload(url, outDir, mp3 = true) { p, line ->
                            mainHandler.post {
                                progress = p
                                append(line)
                            }
                        }
                    }
                },
                enabled = !busy,
            ) { Text("MP3") }
            TextButton(
                onClick = { YoutubeDL.destroyProcessById(PROCESS_ID) },
                enabled = busy,
            ) { Text("Cancel") }
        }
        Text(
            log,
            modifier =
                Modifier
                    .fillMaxWidth()
                    .weight(1f)
                    .verticalScroll(rememberScrollState()),
            style = MaterialTheme.typography.bodySmall,
        )
    }
}

private fun executeDownload(
    url: String,
    outDir: File,
    mp3: Boolean,
    onLine: (Float, String) -> Unit,
): String {
    outDir.mkdirs()
    val request = YoutubeDLRequest(url.trim())
    request.addOption("--no-playlist")
    request.addOption("--newline")
    request.addOption("--no-color")
    request.addOption("-o", File(outDir, "%(title)s.%(ext)s").absolutePath)
    if (mp3) {
        request.addOption("-f", "bestaudio/best")
        request.addOption("-x")
        request.addOption("--audio-format", "mp3")
    }
    val response =
        YoutubeDL.execute(request, PROCESS_ID) { p, _, line ->
            onLine(p, line)
        }
    return "exit ${response.exitCode}\n${response.out}"
}
