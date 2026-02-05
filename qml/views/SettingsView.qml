import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root

    required property var settingsController
    required property var operationController

    ScrollView {
        anchors.fill: parent
        contentWidth: availableWidth
        clip: true

        ColumnLayout {
            width: parent.width
            spacing: 24

            // Header
            Label {
                text: "Settings"
                font.pixelSize: 24
                font.bold: true
                Layout.leftMargin: 24
                Layout.topMargin: 24
            }

            // General settings
            GroupBox {
                title: "General"
                Layout.fillWidth: true
                Layout.leftMargin: 24
                Layout.rightMargin: 24

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 8

                    CheckBox {
                        text: "Check for updates on startup"
                        checked: settingsController.checkUpdatesOnStart
                        onToggled: settingsController.checkUpdatesOnStart = checked
                    }

                    CheckBox {
                        text: "Show Flatpak applications"
                        checked: settingsController.showFlatpak
                        onToggled: settingsController.showFlatpak = checked
                    }

                    CheckBox {
                        text: "Show AUR packages (requires yay/paru)"
                        checked: settingsController.showAurPackages
                        onToggled: settingsController.showAurPackages = checked
                    }
                }
            }

            // Cache settings
            GroupBox {
                title: "Cache Management"
                Layout.fillWidth: true
                Layout.leftMargin: 24
                Layout.rightMargin: 24

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 12

                    RowLayout {
                        Label { text: "Current cache size:" }
                        Label {
                            text: settingsController.cacheSize
                            font.bold: true
                        }
                    }

                    RowLayout {
                        Label { text: "Keep package versions:" }
                        SpinBox {
                            from: 1
                            to: 10
                            value: settingsController.cacheKeepVersions
                            onValueModified: settingsController.cacheKeepVersions = value
                        }
                    }

                    RowLayout {
                        spacing: 8

                        Button {
                            text: "Clean Cache"
                            onClicked: operationController.cleanCache()
                            enabled: !operationController.busy
                        }

                        Button {
                            text: "Remove Orphans"
                            onClicked: operationController.removeOrphans()
                            enabled: !operationController.busy && settingsController.orphanCount > 0
                        }
                    }
                }
            }

            // System maintenance
            GroupBox {
                title: "System Maintenance"
                Layout.fillWidth: true
                Layout.leftMargin: 24
                Layout.rightMargin: 24

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 12

                    Label {
                        text: `Orphan packages: ${settingsController.orphanCount}`
                        color: settingsController.orphanCount > 0 ? "orange" : palette.text
                    }

                    RowLayout {
                        spacing: 8

                        Button {
                            text: "Sync Databases"
                            onClicked: operationController.syncDatabases()
                            enabled: !operationController.busy
                        }

                        Button {
                            text: "Refresh Mirrorlist"
                            enabled: !operationController.busy
                        }
                    }
                }
            }

            // Pacman configuration
            GroupBox {
                title: "Pacman Configuration"
                Layout.fillWidth: true
                Layout.leftMargin: 24
                Layout.rightMargin: 24

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 8

                    Label {
                        text: "Configuration file: /etc/pacman.conf"
                        font.pixelSize: 12
                        opacity: 0.7
                    }

                    Button {
                        text: "Edit pacman.conf"
                        onClicked: {
                            // Open in system editor
                        }
                    }
                }
            }

            // About
            GroupBox {
                title: "About"
                Layout.fillWidth: true
                Layout.leftMargin: 24
                Layout.rightMargin: 24
                Layout.bottomMargin: 24

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 4

                    Label {
                        text: "xPackageManager"
                        font.bold: true
                        font.pixelSize: 16
                    }

                    Label {
                        text: "Version 0.1.0"
                        opacity: 0.7
                    }

                    Label {
                        text: "A modern package manager for Arch Linux"
                        opacity: 0.7
                    }

                    Label {
                        text: "Built with Rust, Qt 6, and CXX-Qt"
                        opacity: 0.7
                        font.pixelSize: 11
                    }
                }
            }
        }
    }
}
