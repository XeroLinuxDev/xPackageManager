import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Dialog {
    id: root

    required property var operationController

    title: "Operation in Progress"
    modal: true
    closePolicy: Popup.NoAutoClose
    anchors.centerIn: parent
    width: 450

    visible: operationController.busy

    ColumnLayout {
        anchors.fill: parent
        spacing: 16

        // Status message
        Label {
            text: operationController.statusMessage
            font.pixelSize: 14
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
        }

        // Current package
        Label {
            visible: operationController.currentPackage !== ""
            text: `Processing: ${operationController.currentPackage}`
            font.pixelSize: 12
            opacity: 0.7
            elide: Text.ElideMiddle
            Layout.fillWidth: true
        }

        // Progress bar
        ProgressBar {
            Layout.fillWidth: true
            value: operationController.progress / 100
            indeterminate: operationController.progress === 0

            background: Rectangle {
                implicitWidth: 400
                implicitHeight: 8
                radius: 4
                color: palette.alternateBase
            }

            contentItem: Item {
                implicitWidth: 400
                implicitHeight: 8

                Rectangle {
                    width: parent.width * root.operationController.progress / 100
                    height: parent.height
                    radius: 4
                    color: palette.highlight
                    visible: !parent.parent.indeterminate

                    Behavior on width {
                        NumberAnimation { duration: 100 }
                    }
                }

                // Indeterminate animation
                Rectangle {
                    visible: parent.parent.indeterminate
                    width: parent.width * 0.3
                    height: parent.height
                    radius: 4
                    color: palette.highlight

                    SequentialAnimation on x {
                        running: parent.visible
                        loops: Animation.Infinite

                        NumberAnimation {
                            from: 0
                            to: parent.parent.width * 0.7
                            duration: 1000
                            easing.type: Easing.InOutQuad
                        }

                        NumberAnimation {
                            from: parent.parent.width * 0.7
                            to: 0
                            duration: 1000
                            easing.type: Easing.InOutQuad
                        }
                    }
                }
            }
        }

        // Progress percentage
        Label {
            visible: operationController.progress > 0
            text: `${operationController.progress}%`
            font.pixelSize: 12
            opacity: 0.7
            Layout.alignment: Qt.AlignHCenter
        }

        // Cancel button
        Button {
            text: "Cancel"
            Layout.alignment: Qt.AlignHCenter
            onClicked: operationController.cancel()
        }
    }
}
