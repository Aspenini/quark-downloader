import QtQuick
import QtQuick.Controls as QQC
import QtQuick.Layouts

QQC.ApplicationWindow {
    id: root
    title: quarkVersion ? "Quark Downloader " + quarkVersion : "Quark Downloader"
    width: 520
    height: 520
    visible: true

    signal submit(string json)
    property string pendingSubmit: ""
    onSubmit: pendingSubmit = json

    property bool settingsSaved: false
    property bool showSettings: false
    property var queue: []
    property string urlText: ""
    property bool audio: false
    property string format: "original"
    property string outDir: outputDir
    property string draftDir: downloadDir
    property string draftMode: guiMode
    property bool draftLogs: logs
    property bool draftOpenOutput: openOutputDir
    property string draftTheme: theme
    property bool draftStrip: stripIds
    property bool draftSanitize: sanitize
    property string draftSpaces: spaces
    property bool draftFolders: playlistFolders

    readonly property var audioFormats: ["original", "mp3", "m4a", "flac", "wav", "opus", "vorbis"]
    readonly property var videoFormats: ["original", "mp4", "mkv", "webm"]

    onClosing: function(close) {
        close.accepted = false
        emitCancel()
    }

    function settingsObject() {
        return {
            download_dir: draftDir,
            yt_dlp: "path",
            ffmpeg: "path",
            gui_download_mode: draftMode,
            download_logs: draftLogs,
            open_output_dir: draftOpenOutput,
            gui_theme: draftTheme,
            strip_video_ids: draftStrip,
            sanitize_filenames: draftSanitize,
            filename_spaces: draftSpaces,
            playlist_folders: draftFolders
        }
    }

    function resetDraft() {
        draftDir = "~/Downloads"
        draftMode = "progress"
        draftLogs = true
        draftOpenOutput = false
        draftTheme = "system"
        draftStrip = true
        draftSanitize = true
        draftSpaces = "keep"
        draftFolders = true
    }

    function emitCancel() {
        var o = { v: 1, action: "cancel" }
        if (settingsSaved)
            o.settings = settingsObject()
        submit(JSON.stringify(o))
    }

    function addUrl(raw) {
        var u = (raw || urlText).trim()
        if (!u)
            return
        if (queue.indexOf(u) < 0)
            queue = queue.concat([u])
        urlText = ""
    }

    QQC.ScrollView {
        id: scroll
        anchors.fill: parent
        anchors.margins: 16
        contentWidth: availableWidth

        Item {
            width: scroll.availableWidth
            implicitHeight: root.showSettings ? settingsColumn.implicitHeight : sessionColumn.implicitHeight

            ColumnLayout {
                id: sessionColumn
                anchors.left: parent.left
                anchors.right: parent.right
                visible: !root.showSettings
                spacing: 10

            QQC.Label { text: "Video or playlist URL"; font.bold: true }
            RowLayout {
                QQC.TextField {
                    Layout.fillWidth: true
                    placeholderText: "https://..."
                    text: root.urlText
                    onTextChanged: root.urlText = text
                    onAccepted: root.addUrl()
                }
                QQC.Button { text: "Add"; onClicked: root.addUrl() }
                QQC.Button {
                    text: "Paste"
                    onClicked: {
                        pasteBuffer.text = ""
                        pasteBuffer.paste()
                        var t = pasteBuffer.text
                        t.split(/\s+/).forEach(function (p) { root.addUrl(p) })
                    }
                }
            }
            QQC.TextField { id: pasteBuffer; visible: false }
            QQC.Label { text: "Queue"; font.bold: true }
            Repeater {
                model: root.queue
                delegate: RowLayout {
                    QQC.Label { Layout.fillWidth: true; text: modelData; elide: Text.ElideMiddle }
                    QQC.Button {
                        text: "Remove"
                        onClicked: {
                            var q = root.queue.slice()
                            q.splice(index, 1)
                            root.queue = q
                        }
                    }
                }
            }
            RowLayout {
                QQC.RadioButton {
                    text: "Video"
                    checked: !root.audio
                    onClicked: { root.audio = false; root.format = "original" }
                }
                QQC.RadioButton {
                    text: "Audio"
                    checked: root.audio
                    onClicked: { root.audio = true; root.format = "original" }
                }
            }
            QQC.Label { text: "Format"; font.bold: true }
            QQC.ComboBox {
                model: root.audio ? root.audioFormats : root.videoFormats
                onActivated: root.format = currentText
            }
            QQC.Label { text: "Output folder"; font.bold: true }
            QQC.TextField {
                Layout.fillWidth: true
                text: root.outDir
                onTextChanged: root.outDir = text
            }
            RowLayout {
                QQC.Button { text: "Settings"; onClicked: root.showSettings = true }
                Item { Layout.fillWidth: true }
                QQC.Button { text: "Cancel"; onClicked: root.emitCancel() }
                QQC.Button {
                    text: "Download"
                    onClicked: {
                        root.addUrl()
                        if (root.queue.length === 0 || !root.outDir.trim())
                            return
                        var o = {
                            v: 1,
                            action: "download",
                            urls: root.queue,
                            media_type: root.audio ? "audio" : "video",
                            format: root.format,
                            output_dir: root.outDir.trim()
                        }
                        if (root.settingsSaved)
                            o.settings = root.settingsObject()
                        root.submit(JSON.stringify(o))
                    }
                }
            }
            }

            ColumnLayout {
                id: settingsColumn
                anchors.left: parent.left
                anchors.right: parent.right
                visible: root.showSettings
                spacing: 10
            QQC.Label { text: "Default download folder"; font.bold: true }
            QQC.TextField { Layout.fillWidth: true; text: root.draftDir; onTextChanged: root.draftDir = text }
            QQC.CheckBox { text: "Remove trailing video ID"; checked: root.draftStrip; onToggled: root.draftStrip = checked }
            QQC.CheckBox { text: "Sanitize filenames"; checked: root.draftSanitize; onToggled: root.draftSanitize = checked }
            QQC.Label { text: "Filename spaces" }
            QQC.ComboBox { model: ["keep", "underscore", "dash", "remove"]; currentIndex: Math.max(0, model.indexOf(root.draftSpaces)); onActivated: root.draftSpaces = currentText }
            QQC.CheckBox { text: "Playlist folders"; checked: root.draftFolders; onToggled: root.draftFolders = checked }
            QQC.Label { text: "Download window" }
            QQC.ComboBox { model: ["progress", "external_cli"]; currentIndex: root.draftMode === "external_cli" ? 1 : 0; onActivated: root.draftMode = currentText }
            QQC.CheckBox { text: "Download logs"; checked: root.draftLogs; onToggled: root.draftLogs = checked }
            QQC.CheckBox { text: "Open output folder when done"; checked: root.draftOpenOutput; onToggled: root.draftOpenOutput = checked }
            QQC.Label { text: "Theme" }
            QQC.ComboBox {
                model: ["system", "light", "dark"]
                currentIndex: Math.max(0, model.indexOf(root.draftTheme))
                onActivated: root.draftTheme = currentText
            }
            RowLayout {
                QQC.Button { text: "Reset to defaults"; onClicked: root.resetDraft() }
                Item { Layout.fillWidth: true }
                QQC.Button { text: "Cancel"; onClicked: root.showSettings = false }
                QQC.Button {
                    text: "Save"
                    onClicked: {
                        if (!root.draftDir.trim())
                            return
                        root.settingsSaved = true
                        root.showSettings = false
                    }
                }
            }
            }
        }
    }
}
