package com.aspenini.quark.ui

import android.app.DownloadManager
import android.content.ClipDescription
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.ContentPaste
import androidx.compose.material.icons.filled.Download
import androidx.compose.material.icons.filled.Folder
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.FilterChip
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.MenuAnchorType

import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.aspenini.quark.UiSnapshot
import com.aspenini.quark.data.Catalog
import com.aspenini.quark.data.QuarkSettings
import com.aspenini.quark.download.DownloadUiState
import kotlinx.coroutines.launch

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun QuarkRoot(
    ui: UiSnapshot,
    download: DownloadUiState,
    onUrl: (String) -> Unit,
    onAdd: () -> Unit,
    onPaste: (String) -> Unit,
    onRemove: (Int) -> Unit,
    onAudio: (Boolean) -> Unit,
    onFormat: (String) -> Unit,
    onDownload: () -> Unit,
    onCancel: () -> Unit,
    onOpenSettings: () -> Unit,
    onCloseSettings: () -> Unit,
    onDraft: (QuarkSettings) -> Unit,
    onReset: () -> Unit,
    onSave: () -> Unit,
    onPickFolder: () -> Unit,
    onUpdateYtDlp: () -> Unit,
    onConsumeSnack: () -> Unit,
) {
    val snackbar = remember { SnackbarHostState() }
    val scope = rememberCoroutineScope()
    LaunchedEffect(ui.snackbar) {
        val msg = ui.snackbar ?: return@LaunchedEffect
        snackbar.showSnackbar(msg)
        onConsumeSnack()
    }
    LaunchedEffect(download.error, download.running, download.saved) {
        if (!download.running && download.status.isNotBlank() && download.queueTotal > 0) {
            snackbar.showSnackbar(download.status)
        }
    }
    Scaffold(
        modifier = Modifier.imePadding(),
        topBar = {
            TopAppBar(
                title = { Text(if (ui.showSettings) "Settings" else "Quark Downloader") },
                navigationIcon = {
                    if (ui.showSettings) {
                        IconButton(onClick = onCloseSettings) {
                            Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                        }
                    }
                },
                actions = {
                    if (!ui.showSettings) {
                        IconButton(onClick = onOpenSettings) {
                            Icon(Icons.Default.Settings, contentDescription = "Settings")
                        }
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(),
            )
        },
        snackbarHost = { SnackbarHost(snackbar) },
        floatingActionButton = {
            if (!ui.showSettings && !download.running) {
                FloatingActionButton(onClick = onDownload) {
                    Icon(Icons.Default.Download, contentDescription = "Download")
                }
            }
        },
    ) { padding ->
        if (ui.showSettings) {
            SettingsScreen(
                padding = padding,
                draft = ui.draft,
                version = ui.ytDlpVersion,
                onDraft = onDraft,
                onReset = onReset,
                onSave = onSave,
                onPickFolder = onPickFolder,
                onUpdateYtDlp = {
                    scope.launch { onUpdateYtDlp() }
                },
            )
        } else {
            MainScreen(
                padding = padding,
                ui = ui,
                download = download,
                onUrl = onUrl,
                onAdd = onAdd,
                onPaste = onPaste,
                onRemove = onRemove,
                onAudio = onAudio,
                onFormat = onFormat,
                onCancel = onCancel,
            )
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
private fun MainScreen(
    padding: PaddingValues,
    ui: UiSnapshot,
    download: DownloadUiState,
    onUrl: (String) -> Unit,
    onAdd: () -> Unit,
    onPaste: (String) -> Unit,
    onRemove: (Int) -> Unit,
    onAudio: (Boolean) -> Unit,
    onFormat: (String) -> Unit,
    onCancel: () -> Unit,
) {
    val context = LocalContext.current
    LazyColumn(
        modifier =
            Modifier
                .fillMaxSize()
                .padding(padding),
        contentPadding = PaddingValues(start = 16.dp, end = 16.dp, bottom = 96.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        item {
            OutlinedTextField(
                value = ui.urlField,
                onValueChange = onUrl,
                modifier = Modifier.fillMaxWidth(),
                label = { Text("Video or playlist URL") },
                placeholder = { Text("https://…") },
                singleLine = true,
                enabled = !download.running,
                keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
                keyboardActions = KeyboardActions(onDone = { onAdd() }),
                trailingIcon = {
                    Row {
                        IconButton(
                            onClick = {
                                val pasted = clipboardText(context)
                                if (pasted.isNotBlank()) onPaste(pasted)
                            },
                            enabled = !download.running,
                        ) {
                            Icon(Icons.Default.ContentPaste, contentDescription = "Paste")
                        }
                        IconButton(onClick = onAdd, enabled = !download.running) {
                            Icon(Icons.Default.Add, contentDescription = "Add")
                        }
                    }
                },
            )
        }
        item {
            Text("Queue", style = MaterialTheme.typography.titleSmall)
        }
        if (ui.queue.isEmpty()) {
            item {
                Text(
                    "Add a URL, paste several, or share a link to Quark.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
        itemsIndexed(ui.queue, key = { _, url -> url }) { index, url ->
            Card {
                ListItem(
                    headlineContent = {
                        Text(url, maxLines = 2, overflow = TextOverflow.Ellipsis)
                    },
                    trailingContent = {
                        IconButton(onClick = { onRemove(index) }, enabled = !download.running) {
                            Icon(Icons.Default.Close, contentDescription = "Remove")
                        }
                    },
                )
            }
        }
        item {
            Text("Type", style = MaterialTheme.typography.titleSmall)
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                FilterChip(
                    selected = !ui.audio,
                    onClick = { onAudio(false) },
                    enabled = !download.running,
                    label = { Text("Video") },
                )
                FilterChip(
                    selected = ui.audio,
                    onClick = { onAudio(true) },
                    enabled = !download.running,
                    label = { Text("Audio") },
                )
            }
        }
        item {
            FormatPicker(
                value = ui.format,
                options = Catalog.formatsFor(ui.audio),
                enabled = !download.running,
                onChange = onFormat,
            )
        }
        item {
            ListItem(
                headlineContent = { Text("Save to") },
                supportingContent = {
                    Text(
                        friendlyDir(ui.output),
                        maxLines = 2,
                        overflow = TextOverflow.Ellipsis,
                    )
                },
                leadingContent = { Icon(Icons.Default.Folder, contentDescription = null) },
            )
        }
        if (download.running || download.status.isNotBlank()) {
            item {
                ProgressCard(download = download, onCancel = onCancel)
            }
        }
        if (download.saved.isNotEmpty() && !download.running) {
            item {
                FilledTonalButton(
                    onClick = {
                        val open = Intent(DownloadManager.ACTION_VIEW_DOWNLOADS)
                        context.startActivity(open)
                    },
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Text("Open Downloads")
                }
            }
        }
    }
}

@Composable
private fun ProgressCard(download: DownloadUiState, onCancel: () -> Unit) {
    Card(
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant),
    ) {
        Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            val label =
                if (download.queueTotal > 1 && download.queueIndex > 0) {
                    "URL ${download.queueIndex} of ${download.queueTotal}"
                } else {
                    if (download.running) "Downloading" else "Finished"
                }
            Text(label, style = MaterialTheme.typography.titleSmall)
            LinearProgressIndicator(
                progress = { (download.percent / 100f).coerceIn(0f, 1f) },
                modifier = Modifier.fillMaxWidth(),
            )
            Text(
                download.status,
                style = MaterialTheme.typography.bodySmall,
                maxLines = 3,
                overflow = TextOverflow.Ellipsis,
            )
            if (download.running) {
                TextButton(onClick = onCancel, modifier = Modifier.align(Alignment.End)) {
                    Text("Cancel")
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun FormatPicker(
    value: String,
    options: List<String>,
    enabled: Boolean,
    onChange: (String) -> Unit,
) {
    var expanded by remember { mutableStateOf(false) }
    ExposedDropdownMenuBox(expanded = expanded, onExpandedChange = { if (enabled) expanded = it }) {
        OutlinedTextField(
            modifier =
                Modifier
                    .menuAnchor(MenuAnchorType.PrimaryNotEditable)
                    .fillMaxWidth(),
            readOnly = true,
            value = value,
            onValueChange = {},
            enabled = enabled,
            label = { Text("Format") },
            trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded) },
        )
        ExposedDropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
            options.forEach { option ->
                DropdownMenuItem(
                    text = { Text(option) },
                    onClick = {
                        onChange(option)
                        expanded = false
                    },
                )
            }
        }
    }
}

@Composable
private fun SettingsScreen(
    padding: PaddingValues,
    draft: QuarkSettings,
    version: String?,
    onDraft: (QuarkSettings) -> Unit,
    onReset: () -> Unit,
    onSave: () -> Unit,
    onPickFolder: () -> Unit,
    onUpdateYtDlp: () -> Unit,
) {
    Column(
        modifier =
            Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text("Default download folder", style = MaterialTheme.typography.titleSmall)
        OutlinedTextField(
            value = draft.downloadDir,
            onValueChange = { onDraft(draft.copy(downloadDir = it)) },
            modifier = Modifier.fillMaxWidth(),
            singleLine = true,
        )
        TextButton(onClick = onPickFolder) { Text("Choose folder") }
        HorizontalDivider()
        Text("Download naming", style = MaterialTheme.typography.titleSmall)
        SettingsSwitch("Remove trailing video ID", draft.stripVideoIds) {
            onDraft(draft.copy(stripVideoIds = it))
        }
        SettingsSwitch("Sanitize filenames", draft.sanitizeFilenames) {
            onDraft(draft.copy(sanitizeFilenames = it))
        }
        ChoiceRow("Filename spaces", Catalog.SPACES, draft.filenameSpaces) {
            onDraft(draft.copy(filenameSpaces = it))
        }
        SettingsSwitch("Playlist folders", draft.playlistFolders) {
            onDraft(draft.copy(playlistFolders = it))
        }
        HorizontalDivider()
        SettingsSwitch("Download logs", draft.downloadLogs) {
            onDraft(draft.copy(downloadLogs = it))
        }
        SettingsSwitch("Open Downloads when finished", draft.openOutputDir) {
            onDraft(draft.copy(openOutputDir = it))
        }
        ChoiceRow("Theme", Catalog.THEMES, draft.guiTheme) {
            onDraft(draft.copy(guiTheme = it))
        }
        HorizontalDivider()
        Text("yt-dlp", style = MaterialTheme.typography.titleSmall)
        Text(
            version ?: "bundled",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        FilledTonalButton(onClick = onUpdateYtDlp, modifier = Modifier.fillMaxWidth()) {
            Text("Check for yt-dlp update")
        }
        Spacer(Modifier.height(8.dp))
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
            TextButton(onClick = onReset) { Text("Reset to defaults") }
            Button(onClick = onSave) { Text("Save") }
        }
    }
}

@Composable
private fun SettingsSwitch(label: String, checked: Boolean, onChange: (Boolean) -> Unit) {
    ListItem(
        headlineContent = { Text(label) },
        trailingContent = { Switch(checked = checked, onCheckedChange = onChange) },
        modifier = Modifier.clickable { onChange(!checked) },
    )
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun ChoiceRow(
    label: String,
    options: List<String>,
    selected: String,
    onChange: (String) -> Unit,
) {
    Text(label, style = MaterialTheme.typography.titleSmall)
    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        options.forEach { option ->
            FilterChip(
                selected = option == selected,
                onClick = { onChange(option) },
                label = { Text(option) },
            )
        }
    }
}

private fun clipboardText(context: Context): String {
    val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
    val clip = clipboard.primaryClip ?: return ""
    if (clip.itemCount < 1) return ""
    if (clipboard.primaryClipDescription?.hasMimeType(ClipDescription.MIMETYPE_TEXT_PLAIN) == false &&
        clipboard.primaryClipDescription?.hasMimeType(ClipDescription.MIMETYPE_TEXT_HTML) == false
    ) {
        return clip.getItemAt(0).coerceToText(context).toString()
    }
    return clip.getItemAt(0).coerceToText(context).toString()
}

fun friendlyDir(path: String): String {
    val downloads = QuarkSettings.publicDownloads().absolutePath
    return if (path == downloads || path.startsWith("$downloads/") || path.startsWith("$downloads\\")) {
        if (path == downloads) "Downloads" else "Downloads / ${path.removePrefix(downloads).trim('/', '\\')}"
    } else {
        path
    }
}


