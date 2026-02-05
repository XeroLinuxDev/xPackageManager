import QtQuick
import QtQuick.Controls

ApplicationWindow {
    id: root
    width: 800
    height: 600
    visible: true
    title: "Test Window"

    Label {
        anchors.centerIn: parent
        text: "Hello from xPackageManager!"
        font.pixelSize: 32
    }
}
