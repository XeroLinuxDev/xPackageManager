import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

TextField {
    id: root

    property alias searchText: root.text
    property string placeholderText: "Search packages..."

    Layout.preferredHeight: 36
    placeholderText: root.placeholderText
    selectByMouse: true

    leftPadding: 36

    // Search icon
    Label {
        anchors.left: parent.left
        anchors.leftMargin: 10
        anchors.verticalCenter: parent.verticalCenter
        text: "🔍"
        font.pixelSize: 14
        opacity: 0.5
    }

    // Clear button
    Button {
        anchors.right: parent.right
        anchors.rightMargin: 4
        anchors.verticalCenter: parent.verticalCenter
        width: 28
        height: 28
        flat: true
        visible: root.text.length > 0
        text: "✕"
        font.pixelSize: 12

        onClicked: {
            root.text = ""
            root.accepted()
        }
    }

    // Debounce timer for search
    Timer {
        id: searchTimer
        interval: 300
        onTriggered: root.searchTextChanged()
    }

    onTextChanged: {
        searchTimer.restart()
    }

    Keys.onEscapePressed: {
        root.text = ""
        root.focus = false
    }
}
