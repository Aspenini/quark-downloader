package com.aspenini.quark

import android.Manifest
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.os.Environment
import android.provider.DocumentsContract
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.activity.viewModels
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.lifecycle.lifecycleScope
import com.aspenini.quark.data.QuarkSettings
import com.aspenini.quark.ui.QuarkRoot
import com.aspenini.quark.ui.QuarkTheme
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.io.File

class MainActivity : ComponentActivity() {
    private val model: QuarkViewModel by viewModels()

    private val folderPicker =
        registerForActivityResult(ActivityResultContracts.OpenDocumentTree()) { uri ->
            if (uri == null) return@registerForActivityResult
            runCatching {
                contentResolver.takePersistableUriPermission(
                    uri,
                    Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION,
                )
            }
            val path = treeToPath(uri) ?: return@registerForActivityResult
            val draft = model.ui.value.draft.copy(downloadDir = path)
            model.updateDraft(draft)
        }

    private val permissionLauncher =
        registerForActivityResult(ActivityResultContracts.RequestMultiplePermissions()) { _ ->
            model.download()
        }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        model.refreshVersion()
        model.ingestIntent(intent)
        setContent {
            val ui by model.ui.collectAsState()
            val download by model.download.collectAsState()
            QuarkTheme(ui.settings.guiTheme) {
                QuarkRoot(
                    ui = ui,
                    download = download,
                    onUrl = model::setUrlField,
                    onAdd = model::addUrl,
                    onPaste = model::paste,
                    onRemove = model::removeAt,
                    onAudio = model::setAudio,
                    onFormat = model::setFormat,
                    onDownload = ::requestAndDownload,
                    onCancel = model::cancelDownload,
                    onOpenSettings = {
                        model.refreshVersion()
                        model.openSettings()
                    },
                    onCloseSettings = model::closeSettings,
                    onDraft = model::updateDraft,
                    onReset = model::resetDraft,
                    onSave = model::saveSettings,
                    onPickFolder = {
                        folderPicker.launch(null)
                    },
                    onUpdateYtDlp = {
                        lifecycleScope.launch {
                            val msg =
                                withContext(Dispatchers.IO) {
                                    runCatching { model.updateYtDlp() }
                                        .getOrElse { e -> "Update failed: ${e.message}" }
                                }
                            model.snack(msg)
                        }
                    },
                    onConsumeSnack = model::consumeSnackbar,
                )
            }
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        model.ingestIntent(intent)
    }

    private fun requestAndDownload() {
        val needed = mutableListOf<String>()
        if (Build.VERSION.SDK_INT >= 33) {
            needed += Manifest.permission.POST_NOTIFICATIONS
        } else if (Build.VERSION.SDK_INT <= 28) {
            needed += Manifest.permission.WRITE_EXTERNAL_STORAGE
        }
        if (needed.isEmpty()) {
            model.download()
            return
        }
        permissionLauncher.launch(needed.toTypedArray())
    }

    private fun treeToPath(uri: Uri): String? {
        if (uri.authority != "com.android.externalstorage.documents") return null
        val docId = DocumentsContract.getTreeDocumentId(uri)
        val split = docId.split(":", limit = 2)
        if (split.size < 2) return null
        if (split[0] != "primary") return null
        val rest = split[1]
        val root = Environment.getExternalStorageDirectory()
        return if (rest.isEmpty()) root.absolutePath else File(root, rest).absolutePath
    }
}
