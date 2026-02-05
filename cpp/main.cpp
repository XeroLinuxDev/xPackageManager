// This file is only needed if we want to use a C++ entry point.
// With CXX-Qt, the Rust main.rs handles application startup.
// This file can be used for Qt Quick Compiler integration if needed.

#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <QQuickStyle>

// The actual entry point is in Rust (crates/xpm-ui/src/main.rs)
// This file is kept for reference and potential future use with
// precompiled QML (Qt Quick Compiler).

/*
int main(int argc, char *argv[])
{
    QGuiApplication app(argc, argv);

    app.setApplicationName("xPackageManager");
    app.setApplicationVersion("0.1.0");
    app.setOrganizationName("xPackageManager");

    // Use Fusion style for consistent cross-platform look
    QQuickStyle::setStyle("Fusion");

    QQmlApplicationEngine engine;

    const QUrl url(QStringLiteral("qrc:/qml/main.qml"));

    QObject::connect(&engine, &QQmlApplicationEngine::objectCreated,
                     &app, [url](QObject *obj, const QUrl &objUrl) {
        if (!obj && url == objUrl)
            QCoreApplication::exit(-1);
    }, Qt::QueuedConnection);

    engine.load(url);

    return app.exec();
}
*/
