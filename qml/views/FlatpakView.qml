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
                text: "Flatpak Applications"
                font.pixelSize: 20
                font.bold: true
            }

            Item { Layout.fillWidth: true }

            Button {
                text: "Manage Remotes"
                onClicked: remotesDialog.open()
            }

            Button {
                text: "Clean Up"
                onClicked: operationController.cleanCache()
                enabled: !operationController.busy
            }
        }

        // Tabs
        TabBar {
            id: tabBar
            Layout.fillWidth: true

            TabButton {
                text: "Installed"
            }

            TabButton {
                text: "Browse Flathub"
            }
        }

        // Content
        StackLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            currentIndex: tabBar.currentIndex

            // Installed flatpaks
            ScrollView {
                clip: true

                ListView {
                    id: flatpakList
                    model: packageModel.count
                    spacing: 8

                    delegate: PackageCard {
                        width: flatpakList.width - 16
                        name: packageModel.getName(index)
                        displayName: packageModel.getDisplayName(index)
                        version: packageModel.getVersion(index)
                        description: packageModel.getDescription(index)
                        repository: packageModel.getRepository(index)
                        backend: 1
                        installed: true
                        hasUpdate: packageModel.hasUpdate(index)
                        selected: packageModel.selectedIndex === index

                        onClicked: packageModel.selectPackage(index)

                        onRemoveClicked: {
                            operationController.removePackage(name, 1)
                        }

                        onUpdateClicked: {
                            operationController.updatePackage(name, 1)
                        }
                    }

                    Label {
                        visible: flatpakList.count === 0 && !packageModel.loading
                        anchors.centerIn: parent
                        text: "No Flatpak applications installed"
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

            // Browse view
            Item {
                ColumnLayout {
                    anchors.fill: parent
                    spacing: 16

                    SearchBar {
                        Layout.preferredWidth: 400
                        Layout.alignment: Qt.AlignHCenter
                        placeholderText: "Search Flathub..."
                    }

                    Label {
                        Layout.alignment: Qt.AlignHCenter
                        text: "Search Flathub for applications to install"
                        opacity: 0.5
                    }
                }
            }
        }
    }

    // Remotes management dialog
    Dialog {
        id: remotesDialog
        title: "Flatpak Remotes"
        width: 500
        height: 400
        anchors.centerIn: parent
        standardButtons: Dialog.Close

        ColumnLayout {
            anchors.fill: parent
            spacing: 8

            ListView {
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true

                model: ListModel {
                    ListElement { name: "flathub"; url: "https://flathub.org/repo/"; enabled: true }
                }

                delegate: Rectangle {
                    width: parent.width
                    height: 56
                    color: palette.base
                    radius: 4

                    RowLayout {
                        anchors.fill: parent
                        anchors.margins: 8

                        CheckBox {
                            checked: model.enabled
                        }

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 2

                            Label {
                                text: model.name
                                font.bold: true
                            }

                            Label {
                                text: model.url
                                font.pixelSize: 11
                                opacity: 0.7
                            }
                        }

                        Button {
                            text: "Remove"
                            flat: true
                            visible: model.name !== "flathub"
                        }
                    }
                }
            }

            Button {
                text: "Add Remote..."
                Layout.alignment: Qt.AlignRight
                onClicked: addRemoteDialog.open()
            }
        }
    }

    // Add remote dialog
    Dialog {
        id: addRemoteDialog
        title: "Add Flatpak Remote"
        anchors.centerIn: parent
        standardButtons: Dialog.Ok | Dialog.Cancel

        ColumnLayout {
            spacing: 8

            Label { text: "Name:" }
            TextField {
                id: remoteNameField
                Layout.preferredWidth: 300
                placeholderText: "e.g., flathub-beta"
            }

            Label { text: "URL:" }
            TextField {
                id: remoteUrlField
                Layout.preferredWidth: 300
                placeholderText: "https://..."
            }
        }

        onAccepted: {
            // settingsController.addFlatpakRemote(remoteNameField.text, remoteUrlField.text)
        }
    }
}
