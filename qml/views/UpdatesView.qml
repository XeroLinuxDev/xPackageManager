import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../components"

Item {
    id: root

    required property var packageModel
    required property var operationController

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 16
        spacing: 8

        // Header
        RowLayout {
            Layout.fillWidth: true

            Label {
                text: "Available Updates"
                font.pixelSize: 20
                font.bold: true
            }

            Item { Layout.fillWidth: true }

            Button {
                text: "Refresh"
                onClicked: packageModel.refresh()
                enabled: !operationController.busy
            }

            Button {
                text: "Update All"
                highlighted: true
                onClicked: operationController.updateAll()
                enabled: !operationController.busy && packageModel.count > 0
            }
        }

        // Summary
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 60
            color: palette.alternateBase
            radius: 8

            RowLayout {
                anchors.fill: parent
                anchors.margins: 16
                spacing: 32

                ColumnLayout {
                    spacing: 2

                    Label {
                        text: packageModel.count
                        font.pixelSize: 24
                        font.bold: true
                    }

                    Label {
                        text: "Updates available"
                        font.pixelSize: 12
                        opacity: 0.7
                    }
                }

                ColumnLayout {
                    spacing: 2

                    Label {
                        text: "~125 MB"
                        font.pixelSize: 24
                        font.bold: true
                    }

                    Label {
                        text: "Download size"
                        font.pixelSize: 12
                        opacity: 0.7
                    }
                }

                Item { Layout.fillWidth: true }
            }
        }

        // Update list
        ScrollView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true

            ListView {
                id: updateList
                model: packageModel.count
                spacing: 8

                delegate: Rectangle {
                    width: updateList.width - 16
                    height: 72
                    color: packageModel.selectedIndex === index ? Qt.lighter(palette.highlight, 1.8) : palette.base
                    radius: 8
                    border.color: packageModel.selectedIndex === index ? palette.highlight : palette.mid
                    border.width: packageModel.selectedIndex === index ? 2 : 1

                    MouseArea {
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: packageModel.selectPackage(index)
                    }

                    RowLayout {
                        anchors.fill: parent
                        anchors.margins: 12
                        spacing: 12

                        // Package icon
                        Rectangle {
                            Layout.preferredWidth: 48
                            Layout.preferredHeight: 48
                            radius: 8
                            color: packageModel.getBackend(index) === 0 ? "#1793D1" : "#4A90D9"

                            Label {
                                anchors.centerIn: parent
                                text: packageModel.getBackend(index) === 0 ? "P" : "F"
                                color: "white"
                                font.pixelSize: 20
                                font.bold: true
                            }
                        }

                        // Package info
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 2

                            Label {
                                text: packageModel.getDisplayName(index)
                                font.bold: true
                                elide: Text.ElideRight
                                Layout.fillWidth: true
                            }

                            Label {
                                text: packageModel.getDescription(index)
                                font.pixelSize: 12
                                opacity: 0.7
                                elide: Text.ElideRight
                                Layout.fillWidth: true
                            }

                            Label {
                                text: `${packageModel.getVersion(index)} → New version`
                                font.pixelSize: 11
                                color: palette.highlight
                            }
                        }

                        // Update button
                        Button {
                            text: "Update"
                            onClicked: operationController.updatePackage(
                                packageModel.getName(index),
                                packageModel.getBackend(index)
                            )
                            enabled: !operationController.busy
                        }
                    }
                }

                // Empty state
                Label {
                    visible: updateList.count === 0 && !packageModel.loading
                    anchors.centerIn: parent
                    text: "System is up to date!"
                    font.pixelSize: 16
                    opacity: 0.5
                }

                BusyIndicator {
                    visible: packageModel.loading
                    anchors.centerIn: parent
                    running: visible
                }
            }
        }
    }
}
