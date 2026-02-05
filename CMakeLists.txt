cmake_minimum_required(VERSION 3.24)
project(xpackagemanager VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)
set(CMAKE_AUTOUIC ON)

# Find Qt6
find_package(Qt6 REQUIRED COMPONENTS Core Gui Qml Quick QuickControls2)

# Find Corrosion for Rust integration
find_package(Corrosion REQUIRED)

# Import the Rust crate
corrosion_import_crate(MANIFEST_PATH Cargo.toml)

# Link Qt libraries to the Rust binary
corrosion_link_libraries(xpm-ui
    Qt6::Core
    Qt6::Gui
    Qt6::Qml
    Qt6::Quick
    Qt6::QuickControls2
)

# Set properties for the executable
corrosion_set_env_vars(xpm-ui
    QT_QPA_PLATFORM=wayland
)

# Install rules
install(TARGETS xpm-ui
    RUNTIME DESTINATION bin
)

# Install QML files for development
install(DIRECTORY qml/
    DESTINATION share/xpackagemanager/qml
)

# Desktop file
configure_file(
    ${CMAKE_SOURCE_DIR}/resources/xpackagemanager.desktop.in
    ${CMAKE_BINARY_DIR}/xpackagemanager.desktop
    @ONLY
)

install(FILES ${CMAKE_BINARY_DIR}/xpackagemanager.desktop
    DESTINATION share/applications
)
