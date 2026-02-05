import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: root

    property string name: ""
    property string displayName: ""
    property string version: ""
    property string description: ""
    property string repository: ""
    property int backend: 0  // 0 = pacman, 1 = flatpak
    property bool installed: false
    property bool hasUpdate: false
    property bool showInstallButton: false
    property bool selected: false

    signal clicked()
    signal installClicked()
    signal removeClicked()
    signal updateClicked()

    height: 80
    color: selected ? Qt.lighter(palette.highlight, 1.8) : (mouseArea.containsMouse ? Qt.lighter(palette.base, 1.05) : palette.base)
    radius: 8
    border.color: selected ? palette.highlight : (hasUpdate ? palette.highlight : palette.mid)
    border.width: selected || hasUpdate ? 2 : 1

    MouseArea {
        id: mouseArea
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor

        onClicked: root.clicked()
    }

    RowLayout {
        anchors.fill: parent
        anchors.margins: 12
        spacing: 12

        // Backend icon
        Rectangle {
            Layout.preferredWidth: 48
            Layout.preferredHeight: 48
            radius: 8
            color: backend === 0 ? "#1793D1" : "#4A90D9"

            Label {
                anchors.centerIn: parent
                text: backend === 0 ? "P" : "F"
                color: "white"
                font.pixelSize: 20
                font.bold: true
            }
        }

        // Package info
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 2

            RowLayout {
                spacing: 8

                Label {
                    text: root.displayName || root.name
                    font.bold: true
                    font.pixelSize: 14
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                }

                // Update badge
                Rectangle {
                    visible: hasUpdate
                    width: updateLabel.width + 12
                    height: 18
                    radius: 9
                    color: palette.highlight

                    Label {
                        id: updateLabel
                        anchors.centerIn: parent
                        text: "Update"
                        font.pixelSize: 10
                        color: palette.highlightedText
                    }
                }
            }

            Label {
                text: root.description
                font.pixelSize: 12
                opacity: 0.7
                elide: Text.ElideRight
                Layout.fillWidth: true
            }

            RowLayout {
                spacing: 16

                Label {
                    text: root.version
                    font.pixelSize: 11
                    opacity: 0.6
                }

                Label {
                    text: root.repository
                    font.pixelSize: 11
                    opacity: 0.6
                }

                Label {
                    text: backend === 0 ? "Pacman" : "Flatpak"
                    font.pixelSize: 11
                    opacity: 0.6
                }
            }
        }

        // Action buttons
        RowLayout {
            spacing: 4

            Button {
                visible: showInstallButton && !installed
                text: "Install"
                highlighted: true
                onClicked: root.installClicked()
            }

            Button {
                visible: hasUpdate
                text: "Update"
                highlighted: true
                onClicked: root.updateClicked()
            }

            Button {
                visible: installed
                text: "Remove"
                flat: true
                onClicked: root.removeClicked()
            }
        }
    }
}
