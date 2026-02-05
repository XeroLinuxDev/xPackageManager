import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: root

    required property var packageModel
    required property var operationController

    property bool hasSelection: packageModel.selectedIndex >= 0

    color: palette.base
    border.color: palette.mid
    border.width: 1

    // No selection state
    Label {
        visible: !hasSelection
        anchors.centerIn: parent
        text: "Select a package to view details"
        opacity: 0.5
        font.pixelSize: 14
    }

    // Package details
    ScrollView {
        visible: hasSelection
        anchors.fill: parent
        anchors.margins: 1
        clip: true
        contentWidth: availableWidth

        ColumnLayout {
            width: parent.width
            spacing: 0

            // Header
            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: headerContent.height + 24
                color: palette.alternateBase

                ColumnLayout {
                    id: headerContent
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.margins: 12
                    spacing: 4

                    RowLayout {
                        spacing: 12

                        // Backend icon
                        Rectangle {
                            width: 48
                            height: 48
                            radius: 8
                            color: packageModel.getSelectedBackend() === 0 ? "#1793D1" : "#4A90D9"

                            Label {
                                anchors.centerIn: parent
                                text: packageModel.getSelectedBackend() === 0 ? "P" : "F"
                                color: "white"
                                font.pixelSize: 20
                                font.bold: true
                            }
                        }

                        ColumnLayout {
                            spacing: 2

                            Label {
                                text: packageModel.getSelectedDisplayName()
                                font.pixelSize: 18
                                font.bold: true
                            }

                            Label {
                                text: packageModel.getSelectedName()
                                font.pixelSize: 11
                                opacity: 0.6
                                visible: packageModel.getSelectedName() !== packageModel.getSelectedDisplayName()
                            }
                        }
                    }

                    Label {
                        text: packageModel.getSelectedDescription()
                        wrapMode: Text.WordWrap
                        Layout.fillWidth: true
                        opacity: 0.8
                    }

                    // Action buttons
                    RowLayout {
                        Layout.topMargin: 8
                        spacing: 8

                        Button {
                            text: packageModel.isSelectedInstalled() ? "Remove" : "Install"
                            highlighted: !packageModel.isSelectedInstalled()
                            onClicked: {
                                if (packageModel.isSelectedInstalled()) {
                                    operationController.removePackage(
                                        packageModel.getSelectedName(),
                                        packageModel.getSelectedBackend()
                                    )
                                } else {
                                    operationController.installPackage(
                                        packageModel.getSelectedName(),
                                        packageModel.getSelectedBackend()
                                    )
                                }
                            }
                            enabled: !operationController.busy
                        }

                        Button {
                            text: "Open Website"
                            visible: packageModel.getSelectedUrl() !== ""
                            onClicked: Qt.openUrlExternally(packageModel.getSelectedUrl())
                        }
                    }
                }
            }

            // Info sections
            ColumnLayout {
                Layout.fillWidth: true
                Layout.margins: 12
                spacing: 16

                // Basic info
                InfoSection {
                    title: "Information"
                    Layout.fillWidth: true

                    GridLayout {
                        columns: 2
                        columnSpacing: 16
                        rowSpacing: 4
                        Layout.fillWidth: true

                        InfoLabel { label: "Version"; value: packageModel.getSelectedVersion() }
                        InfoLabel { label: "Repository"; value: packageModel.getSelectedRepository() }
                        InfoLabel { label: "Architecture"; value: packageModel.getSelectedArch() }
                        InfoLabel { label: "Installed Size"; value: packageModel.getSelectedInstalledSize() }
                        InfoLabel {
                            label: "Download Size"
                            value: packageModel.getSelectedDownloadSize()
                            visible: packageModel.getSelectedDownloadSize() !== "0 B"
                        }
                        InfoLabel {
                            label: "Install Date"
                            value: packageModel.getSelectedInstallDate()
                            visible: packageModel.isSelectedInstalled()
                        }
                        InfoLabel {
                            label: "Licenses"
                            value: packageModel.getSelectedLicenses()
                            visible: packageModel.getSelectedLicenses() !== ""
                        }
                        InfoLabel {
                            label: "Groups"
                            value: packageModel.getSelectedGroups()
                            visible: packageModel.getSelectedGroups() !== ""
                        }
                        InfoLabel {
                            label: "Packager"
                            value: packageModel.getSelectedPackager()
                            visible: packageModel.getSelectedPackager() !== ""
                        }
                    }
                }

                // Dependencies
                InfoSection {
                    title: "Dependencies"
                    visible: packageModel.getSelectedDependencies() !== ""
                    Layout.fillWidth: true

                    ColumnLayout {
                        spacing: 2
                        Layout.fillWidth: true

                        Repeater {
                            model: packageModel.getSelectedDependencies().split("\n")
                            delegate: Label {
                                text: modelData
                                font.pixelSize: 12
                                color: palette.link
                                MouseArea {
                                    anchors.fill: parent
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: {
                                        // TODO: Navigate to dependency
                                    }
                                }
                            }
                        }
                    }
                }

                // Optional dependencies
                InfoSection {
                    title: "Optional Dependencies"
                    visible: packageModel.getSelectedOptionalDeps() !== ""
                    Layout.fillWidth: true

                    ColumnLayout {
                        spacing: 2
                        Layout.fillWidth: true

                        Repeater {
                            model: packageModel.getSelectedOptionalDeps().split("\n")
                            delegate: Label {
                                text: modelData
                                font.pixelSize: 12
                                opacity: 0.8
                                wrapMode: Text.WordWrap
                                Layout.fillWidth: true
                            }
                        }
                    }
                }

                // Required by
                InfoSection {
                    title: "Required By"
                    visible: packageModel.getSelectedRequiredBy() !== ""
                    Layout.fillWidth: true

                    ColumnLayout {
                        spacing: 2
                        Layout.fillWidth: true

                        Repeater {
                            model: packageModel.getSelectedRequiredBy().split("\n")
                            delegate: Label {
                                text: modelData
                                font.pixelSize: 12
                                color: palette.link
                            }
                        }
                    }
                }

                // Provides
                InfoSection {
                    title: "Provides"
                    visible: packageModel.getSelectedProvides() !== ""
                    Layout.fillWidth: true

                    ColumnLayout {
                        spacing: 2
                        Layout.fillWidth: true

                        Repeater {
                            model: packageModel.getSelectedProvides().split("\n")
                            delegate: Label {
                                text: modelData
                                font.pixelSize: 12
                            }
                        }
                    }
                }

                // Conflicts
                InfoSection {
                    title: "Conflicts"
                    visible: packageModel.getSelectedConflicts() !== ""
                    Layout.fillWidth: true

                    ColumnLayout {
                        spacing: 2
                        Layout.fillWidth: true

                        Repeater {
                            model: packageModel.getSelectedConflicts().split("\n")
                            delegate: Label {
                                text: modelData
                                font.pixelSize: 12
                                color: "orange"
                            }
                        }
                    }
                }

                Item { Layout.fillHeight: true }
            }
        }
    }

    // Info section component
    component InfoSection: ColumnLayout {
        property string title: ""
        default property alias content: contentItem.data

        spacing: 8

        Label {
            text: title
            font.bold: true
            font.pixelSize: 13
        }

        Rectangle {
            Layout.fillWidth: true
            height: 1
            color: palette.mid
        }

        Item {
            id: contentItem
            Layout.fillWidth: true
            implicitHeight: childrenRect.height
        }
    }

    // Info label component
    component InfoLabel: RowLayout {
        property string label: ""
        property string value: ""

        spacing: 8
        Layout.fillWidth: true

        Label {
            text: label + ":"
            font.pixelSize: 12
            opacity: 0.7
            Layout.preferredWidth: 100
        }

        Label {
            text: value
            font.pixelSize: 12
            wrapMode: Text.WordWrap
            Layout.fillWidth: true
        }
    }
}
