import QtQuick
import QtQuick.Controls as QQC
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

Kirigami.ApplicationWindow {
    id: root
    title: quarkVersion ? "Quark Downloader " + quarkVersion : "Quark Downloader"
    width: 520
    height: 520
    visible: true

    signal submit(string json)
    signal closed()

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
    property string draftTheme: theme
    property bool draftStrip: stripIds
    property bool draftSanitize: sanitize
    property string draftSpaces: spaces
    property bool draftFolders: playlistFolders
    property string draftFrontend: frontend

    readonly property var audioFormats: ["original", "mp3", "m4a", "flac", "wav", "opus", "vorbis"]
    readonly property var videoFormats: ["original", "mp4", "mkv", "webm"]
    readonly property var frontendChoices: ["auto", "cosmic", "kirigami"]

    onClosing: {
        emitCancel()
        closed()
    }

    function settingsObject() {
        return {
            download_dir: draftDir,
            yt_dlp: "path",
            ffmpeg: "path",
            gui_download_mode: draftMode,
            download_logs: draftLogs,
            gui_theme: draftTheme,
            strip_video_ids: draftStrip,
            sanitize_filenames: draftSanitize,
            filename_spaces: draftSpaces,
            playlist_folders: draftFolders,
            gui_frontend: draftFrontend
        }
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

    pageStack.initialPage: Kirigami.ScrollablePage {
        title: root.showSettings ? "Settings" : root.title

        ColumnLayout {
            visible: !root.showSettings
            spacing: Kirigami.Units.smallSpacing

            Kirigami.Heading { text: "Video or playlist URL"; level: 4 }
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
                        var t = ""
                        try { t = clipboard.text() } catch (e) {}
                        t.split(/\s+/).forEach(function (p) { root.addUrl(p) })
                    }
                }
            }
            Kirigami.Heading { text: "Queue"; level: 4 }
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
            Kirigami.Heading { text: "Format"; level: 4 }
            QQC.ComboBox {
                model: root.audio ? root.audioFormats : root.videoFormats
                onActivated: root.format = currentText
            }
            Kirigami.Heading { text: "Output folder"; level: 4 }
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
            visible: root.showSettings
            spacing: Kirigami.Units.smallSpacing
            Kirigami.Heading { text: "Default download folder"; level: 4 }
            QQC.TextField { Layout.fillWidth: true; text: root.draftDir; onTextChanged: root.draftDir = text }
            QQC.CheckBox { text: "Remove trailing video ID"; checked: root.draftStrip; onToggled: root.draftStrip = checked }
            QQC.CheckBox { text: "Sanitize filenames"; checked: root.draftSanitize; onToggled: root.draftSanitize = checked }
            QQC.Label { text: "Filename spaces" }
            QQC.ComboBox { model: ["keep", "underscore", "dash", "remove"]; currentIndex: Math.max(0, model.indexOf(root.draftSpaces)); onActivated: root.draftSpaces = currentText }
            QQC.CheckBox { text: "Playlist folders"; checked: root.draftFolders; onToggled: root.draftFolders = checked }
            QQC.Label { text: "Download window" }
            QQC.ComboBox { model: ["progress", "external_cli"]; currentIndex: root.draftMode === "external_cli" ? 1 : 0; onActivated: root.draftMode = currentText }
            QQC.CheckBox { text: "Download logs"; checked: root.draftLogs; onToggled: root.draftLogs = checked }
            QQC.Label { text: "Theme" }
            QQC.ComboBox {
                model: ["system", "light", "dark"]
                Component.onCompleted: currentIndex = Math.max(0, model.indexOf(root.draftTheme))
                onActivated: root.draftTheme = currentText
            }
            QQC.Label { text: "GUI frontend" }
            QQC.ComboBox {
                model: root.frontendChoices
                Component.onCompleted: currentIndex = Math.max(0, model.indexOf(root.draftFrontend))
                onActivated: root.draftFrontend = currentText
            }
            RowLayout {
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
