import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import xpm

ApplicationWindow {
    id: root
    width: 1400
    height: 900
    minimumWidth: 1000
    minimumHeight: 700
    visible: true
    title: qsTr("xPackageManager")

    // Use system palette for native theming
    palette: SystemPalette { colorGroup: SystemPalette.Active }

    // Controllers
    PackageModel {
        id: packageModel
        Component.onCompleted: refresh()
    }

    OperationController {
        id: operationController
        onOperationCompleted: (message) => {
            statusMessage.text = message
            packageModel.refresh()
        }
        onOperationFailed: (error) => {
            errorDialog.text = error
            errorDialog.open()
        }
    }

    SettingsController {
        id: settingsController
        Component.onCompleted: {
            loadSettings()
            refreshStats()
        }
    }

    // Main layout
    RowLayout {
        anchors.fill: parent
        spacing: 0

        // Sidebar
        Rectangle {
            Layout.preferredWidth: 200
            Layout.fillHeight: true
            color: palette.alternateBase

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 8
                spacing: 4

                // Logo/Title
                RowLayout {
                    Layout.alignment: Qt.AlignHCenter
                    Layout.topMargin: 12
                    Layout.bottomMargin: 12
                    spacing: 8

                    Rectangle {
                        width: 32
                        height: 32
                        radius: 6
                        color: "#1793D1"

                        Label {
                            anchors.centerIn: parent
                            text: "X"
                            color: "white"
                            font.pixelSize: 18
                            font.bold: true
                        }
                    }

                    Label {
                        text: "xPM"
                        font.pixelSize: 20
                        font.bold: true
                    }
                }

                // Navigation buttons
                Repeater {
                    model: [
                        { text: "Installed", view: 0 },
                        { text: "Updates", view: 1, badge: settingsController.updateCount },
                        { text: "Search", view: 2 },
                        { text: "Flatpak", view: 3 },
                        { text: "Settings", view: 4 }
                    ]

                    delegate: Button {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 40
                        flat: true
                        highlighted: stackView.currentIndex === modelData.view

                        contentItem: RowLayout {
                            spacing: 8

                            Label {
                                text: modelData.text
                                Layout.fillWidth: true
                                font.weight: stackView.currentIndex === modelData.view ? Font.DemiBold : Font.Normal
                            }

                            Rectangle {
                                visible: modelData.badge > 0
                                width: badgeText.width + 12
                                height: 18
                                radius: 9
                                color: palette.highlight

                                Label {
                                    id: badgeText
                                    anchors.centerIn: parent
                                    text: modelData.badge
                                    font.pixelSize: 10
                                    color: palette.highlightedText
                                }
                            }
                        }

                        onClicked: {
                            stackView.currentIndex = modelData.view
                            packageModel.setView(modelData.view)
                            packageModel.refresh()
                        }
                    }
                }

                Item { Layout.fillHeight: true }

                // Stats section
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: statsColumn.height + 16
                    color: palette.base
                    radius: 6

                    ColumnLayout {
                        id: statsColumn
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.top: parent.top
                        anchors.margins: 8
                        spacing: 3

                        Label {
                            text: "Statistics"
                            font.bold: true
                            font.pixelSize: 11
                        }

                        Label {
                            text: `${settingsController.pacmanCount} pacman packages`
                            font.pixelSize: 10
                            opacity: 0.7
                        }

                        Label {
                            text: `${settingsController.flatpakCount} flatpak apps`
                            font.pixelSize: 10
                            opacity: 0.7
                        }

                        Label {
                            text: `${settingsController.orphanCount} orphans`
                            font.pixelSize: 10
                            opacity: 0.7
                            color: settingsController.orphanCount > 0 ? "#e67e22" : palette.text
                        }

                        Label {
                            text: `${settingsController.cacheSize} cache`
                            font.pixelSize: 10
                            opacity: 0.7
                        }
                    }
                }
            }
        }

        // Separator
        Rectangle {
            Layout.preferredWidth: 1
            Layout.fillHeight: true
            color: palette.mid
        }

        // Main content area
        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0

            // Toolbar
            ToolBar {
                Layout.fillWidth: true
                background: Rectangle {
                    color: palette.window
                    Rectangle {
                        anchors.bottom: parent.bottom
                        width: parent.width
                        height: 1
                        color: palette.mid
                    }
                }

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 12
                    anchors.rightMargin: 12

                    SearchBar {
                        id: searchBar
                        Layout.preferredWidth: 280
                        onSearchTextChanged: {
                            if (stackView.currentIndex !== 2) {
                                stackView.currentIndex = 2
                            }
                            packageModel.search(searchText)
                        }
                    }

                    Item { Layout.fillWidth: true }

                    Button {
                        text: "Sync"
                        flat: true
                        onClicked: operationController.syncDatabases()
                        enabled: !operationController.busy
                    }

                    Button {
                        text: "Update All"
                        highlighted: settingsController.updateCount > 0
                        onClicked: operationController.updateAll()
                        enabled: !operationController.busy && settingsController.updateCount > 0
                    }
                }
            }

            // Content area with info pane
            SplitView {
                Layout.fillWidth: true
                Layout.fillHeight: true
                orientation: Qt.Horizontal

                // Main content
                StackLayout {
                    id: stackView
                    SplitView.fillWidth: true
                    SplitView.minimumWidth: 400
                    currentIndex: 0

                    InstalledView {
                        packageModel: packageModel
                        operationController: operationController
                    }

                    UpdatesView {
                        packageModel: packageModel
                        operationController: operationController
                    }

                    InstalledView {
                        packageModel: packageModel
                        operationController: operationController
                        showInstallButton: true
                    }

                    FlatpakView {
                        packageModel: packageModel
                        operationController: operationController
                    }

                    SettingsView {
                        settingsController: settingsController
                        operationController: operationController
                    }
                }

                // Info pane (hidden on settings view)
                PackageInfoPane {
                    visible: stackView.currentIndex !== 4
                    SplitView.preferredWidth: 350
                    SplitView.minimumWidth: 280
                    SplitView.maximumWidth: 500
                    packageModel: packageModel
                    operationController: operationController
                }
            }

            // Status bar
            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 26
                color: palette.alternateBase

                Rectangle {
                    anchors.top: parent.top
                    width: parent.width
                    height: 1
                    color: palette.mid
                }

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 12
                    anchors.rightMargin: 12

                    Label {
                        id: statusMessage
                        text: operationController.busy ? operationController.statusMessage : "Ready"
                        font.pixelSize: 11
                        opacity: 0.7
                    }

                    Item { Layout.fillWidth: true }

                    ProgressBar {
                        visible: operationController.busy
                        Layout.preferredWidth: 120
                        value: operationController.progress / 100
                        indeterminate: operationController.progress === 0
                    }

                    Label {
                        visible: operationController.busy
                        text: operationController.currentPackage
                        font.pixelSize: 11
                        opacity: 0.7
                    }
                }
            }
        }
    }

    // Progress dialog
    ProgressDialog {
        id: progressDialog
        operationController: operationController
    }

    // Error dialog
    Dialog {
        id: errorDialog
        property alias text: errorLabel.text
        title: "Error"
        standardButtons: Dialog.Ok
        anchors.centerIn: parent
        modal: true

        Label {
            id: errorLabel
            wrapMode: Text.WordWrap
        }
    }
}
