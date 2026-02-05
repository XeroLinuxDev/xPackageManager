import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import "../components"

Item {
    id: root

    required property var packageModel
    required property var operationController
    property bool showInstallButton: false

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 16
        spacing: 8

        // Header
        RowLayout {
            Layout.fillWidth: true

            Label {
                text: showInstallButton ? "Search Results" : "Installed Packages"
                font.pixelSize: 20
                font.bold: true
            }

            Item { Layout.fillWidth: true }

            Label {
                text: `${packageModel.count} packages`
                opacity: 0.7
            }
        }

        // Filter bar
        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            ComboBox {
                id: backendFilter
                model: ["All", "Pacman", "Flatpak"]
                onCurrentIndexChanged: {
                    // TODO: Apply filter
                }
            }

            ComboBox {
                id: sortBy
                model: ["Name", "Size", "Install Date"]
            }

            Item { Layout.fillWidth: true }
        }

        // Package list
        ScrollView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true

            ListView {
                id: packageList
                model: packageModel.count
                spacing: 8

                delegate: PackageCard {
                    width: packageList.width - 16
                    name: packageModel.getName(index)
                    displayName: packageModel.getDisplayName(index)
                    version: packageModel.getVersion(index)
                    description: packageModel.getDescription(index)
                    repository: packageModel.getRepository(index)
                    backend: packageModel.getBackend(index)
                    installed: packageModel.isInstalled(index)
                    hasUpdate: packageModel.hasUpdate(index)
                    showInstallButton: root.showInstallButton && !installed
                    selected: packageModel.selectedIndex === index

                    onClicked: packageModel.selectPackage(index)

                    onInstallClicked: {
                        operationController.installPackage(name, backend)
                    }

                    onRemoveClicked: {
                        removeConfirmDialog.packageName = name
                        removeConfirmDialog.packageBackend = backend
                        removeConfirmDialog.open()
                    }

                    onUpdateClicked: {
                        operationController.updatePackage(name, backend)
                    }
                }

                // Empty state
                Label {
                    visible: packageList.count === 0 && !packageModel.loading
                    anchors.centerIn: parent
                    text: root.showInstallButton ? "No packages found" : "No packages installed"
                    font.pixelSize: 16
                    opacity: 0.5
                }

                // Loading indicator
                BusyIndicator {
                    visible: packageModel.loading
                    anchors.centerIn: parent
                    running: visible
                }
            }
        }
    }

    // Remove confirmation dialog
    Dialog {
        id: removeConfirmDialog
        property string packageName: ""
        property int packageBackend: 0

        title: "Remove Package"
        standardButtons: Dialog.Yes | Dialog.No
        anchors.centerIn: parent

        Label {
            text: `Are you sure you want to remove "${removeConfirmDialog.packageName}"?`
            wrapMode: Text.WordWrap
        }

        onAccepted: {
            operationController.removePackage(packageName, packageBackend)
        }
    }
}
