import QtQuick
import QtQuick.Controls as QQC
import org.kde.kirigami as Kirigami

Kirigami.ApplicationWindow {
    id: root
    title: msgTitle
    width: 420
    height: 180
    visible: true

    signal submit(string json)
    signal closed()

    onClosing: closed()

    pageStack.initialPage: Kirigami.Page {
        Column {
            anchors.centerIn: parent
            width: parent.width - 32
            spacing: 12
            Kirigami.Heading { text: msgTitle; level: 2 }
            QQC.Label { text: msgBody; wrapMode: Text.WordWrap; width: parent.width }
            QQC.Button { text: "OK"; onClicked: Qt.quit() }
        }
    }
}
