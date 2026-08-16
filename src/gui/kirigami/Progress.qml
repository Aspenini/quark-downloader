import QtQuick
import QtQuick.Controls as QQC
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

Kirigami.ApplicationWindow {
    id: root
    title: quarkVersion ? "Quark Downloader " + quarkVersion : "Quark Downloader"
    width: 480
    height: 180
    visible: true

    signal submit(string json)
    signal closed()
    property string pendingSubmit: ""
    property bool pendingClose: false
    onSubmit: pendingSubmit = json
    onClosed: pendingClose = true

    property string statusText: "Starting download..."
    property string etaText: ""
    property string queueText: ""
    property real fraction: 0

    function applyLine(line) {
        var i = line.indexOf("\t")
        var kind = i < 0 ? line : line.substring(0, i)
        var rest = i < 0 ? "" : line.substring(i + 1)
        if (kind === "PROGRESS")
            fraction = Number(rest) / 100
        else if (kind === "STATUS")
            statusText = rest
        else if (kind === "ETA")
            etaText = rest
        else if (kind === "QUEUE")
            queueText = rest
        else if (kind === "DONE")
            Qt.quit()
    }

    onClosing: closed()

    pageStack.initialPage: Kirigami.Page {
        ColumnLayout {
            anchors.fill: parent
            Kirigami.Heading { text: root.queueText; level: 4; visible: root.queueText.length > 0 }
            QQC.Label { text: root.statusText; wrapMode: Text.WordWrap; Layout.fillWidth: true }
            QQC.ProgressBar { Layout.fillWidth: true; value: root.fraction }
            QQC.Label { text: root.etaText.length ? "Time left: " + root.etaText : "Time left: estimating..." }
        }
    }
}
