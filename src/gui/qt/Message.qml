import QtQuick
import QtQuick.Controls as QQC

QQC.ApplicationWindow {
    id: root
    title: msgTitle
    width: 420
    height: 180
    visible: true

    Column {
        anchors.centerIn: parent
        width: parent.width - 32
        spacing: 12
        QQC.Label { text: msgTitle; font.bold: true; font.pixelSize: 20 }
        QQC.Label { text: msgBody; wrapMode: Text.WordWrap; width: parent.width }
        QQC.Button { text: "OK"; onClicked: Qt.quit() }
    }
}
