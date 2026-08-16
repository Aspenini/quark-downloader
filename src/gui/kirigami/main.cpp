// Quark Downloader — Kirigami helper. Dynamically links system Qt 6.
// Kirigami itself is a QML module provided by the distro.
#include <QGuiApplication>
#include <QStyleHints>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QSocketNotifier>
#include <QFile>
#include <QDir>
#include <QByteArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonArray>
#include <QTimer>
#include <cstdio>
#include <cstring>

static QString qmlDir()
{
    const QString fromEnv = qEnvironmentVariable("QUARK_KIRIGAMI_QML");
    if (!fromEnv.isEmpty() && QDir(fromEnv).exists())
        return fromEnv;
    const QString sibling = QCoreApplication::applicationDirPath() + "/qml";
    if (QDir(sibling).exists())
        return sibling;
#ifdef QUARK_KIRIGAMI_QML
    {
        const QString baked = QString::fromUtf8(QUARK_KIRIGAMI_QML);
        if (QDir(baked).exists())
            return baked;
    }
#endif
    return QCoreApplication::applicationDirPath();
}

static void applyColorScheme(const QString &theme)
{
#if QT_VERSION >= QT_VERSION_CHECK(6, 5, 0)
    const QString t = theme.toLower();
    if (t == QLatin1String("dark"))
        QGuiApplication::styleHints()->setColorScheme(Qt::ColorScheme::Dark);
    else if (t == QLatin1String("light"))
        QGuiApplication::styleHints()->setColorScheme(Qt::ColorScheme::Light);
    else
        QGuiApplication::styleHints()->setColorScheme(Qt::ColorScheme::Unknown);
#else
    Q_UNUSED(theme);
#endif
}

static void writeJson(const QJsonObject &o)
{
    const QByteArray bytes = QJsonDocument(o).toJson(QJsonDocument::Compact);
    fwrite(bytes.constData(), 1, size_t(bytes.size()), stdout);
    fputc('\n', stdout);
    fflush(stdout);
}

static QJsonObject settingsFromCtx(QQmlContext *ctx)
{
    auto s = [&](const char *k) { return ctx->contextProperty(k).toString(); };
    auto b = [&](const char *k) { return ctx->contextProperty(k).toBool(); };
    return QJsonObject{
        {"download_dir", s("downloadDir")},
        {"yt_dlp", "path"},
        {"ffmpeg", "path"},
        {"gui_download_mode", s("guiMode")},
        {"download_logs", b("logs")},
        {"gui_theme", s("theme")},
        {"strip_video_ids", b("stripIds")},
        {"sanitize_filenames", b("sanitize")},
        {"filename_spaces", s("spaces")},
        {"playlist_folders", b("playlistFolders")},
        {"gui_frontend", s("frontend")},
    };
}

extern "C" int kirigami_ui_run(int argc, char **argv)
{
    if (argc < 2) {
        fprintf(stderr, "usage: --session|--progress|--message\n");
        return 2;
    }
    // Copy before QGuiApplication, which may mutate argv.
    const QByteArray modeBytes = QByteArray(argv[1]);
    const char *mode = modeBytes.constData();

    qputenv("QT_QUICK_CONTROLS_STYLE", "org.kde.desktop");
    QGuiApplication app(argc, argv);

    QQmlApplicationEngine engine;
    QQmlContext *ctx = engine.rootContext();
    ctx->setContextProperty("quarkVersion", qEnvironmentVariable("QUARK_VERSION"));
    ctx->setContextProperty("quarkMode", QString::fromUtf8(mode));

    auto arg = [&](int i, const QString &fb) -> QString {
        return (i + 2 < argc) ? QString::fromLocal8Bit(argv[i + 2]) : fb;
    };
    auto barg = [&](int i, bool fb) {
        if (i + 2 >= argc)
            return fb;
        const QString v = QString::fromLocal8Bit(argv[i + 2]).toLower();
        return v == "1" || v == "true" || v == "yes" || v == "on";
    };

    const QString defaultDir = arg(0, QDir::homePath() + "/Downloads");
    ctx->setContextProperty("defaultDir", defaultDir);
    ctx->setContextProperty("downloadDir", arg(1, QStringLiteral("~/Downloads")));
    ctx->setContextProperty("guiMode", arg(4, QStringLiteral("progress")));
    ctx->setContextProperty("logs", barg(5, true));
    ctx->setContextProperty("theme", arg(6, QStringLiteral("system")));
    applyColorScheme(ctx->contextProperty("theme").toString());
    ctx->setContextProperty("stripIds", barg(7, true));
    ctx->setContextProperty("sanitize", barg(8, true));
    ctx->setContextProperty("spaces", arg(9, QStringLiteral("keep")));
    ctx->setContextProperty("playlistFolders", barg(10, true));
    ctx->setContextProperty("frontend", arg(11, QStringLiteral("auto")));
    ctx->setContextProperty("outputDir", defaultDir);

    const QString qmlName = strcmp(mode, "--progress") == 0
        ? QStringLiteral("Progress.qml")
        : strcmp(mode, "--message") == 0
            ? QStringLiteral("Message.qml")
            : QStringLiteral("Session.qml");
    if (strcmp(mode, "--message") == 0) {
        ctx->setContextProperty("msgKind", argc > 2 ? QString::fromLocal8Bit(argv[2]) : QStringLiteral("ok"));
        ctx->setContextProperty("msgTitle", argc > 3 ? QString::fromLocal8Bit(argv[3]) : QStringLiteral("Quark Downloader"));
        QString body;
        for (int i = 4; i < argc; ++i) {
            if (i > 4)
                body += ' ';
            body += QString::fromLocal8Bit(argv[i]);
        }
        ctx->setContextProperty("msgBody", body);
    }

    const QString qmlPath = qmlDir() + "/" + qmlName;
    if (!QFile::exists(qmlPath)) {
        fprintf(stderr, "missing QML %s\n", qPrintable(qmlPath));
        return 1;
    }
    engine.load(QUrl::fromLocalFile(qmlPath));
    if (engine.rootObjects().isEmpty())
        return 1;
    QObject *root = engine.rootObjects().constFirst();

    // Qt 6 cannot mix SIGNAL() with a lambda. QML writes pendingSubmit /
    // pendingClose; a typed QTimer slot reads them.
    auto *timer = new QTimer(&app);
    timer->setInterval(20);
    QObject::connect(timer, &QTimer::timeout, &app, [root]() {
        const QString json = root->property("pendingSubmit").toString();
        if (!json.isEmpty()) {
            root->setProperty("pendingSubmit", QString());
            const QByteArray bytes = json.toUtf8();
            fwrite(bytes.constData(), 1, size_t(bytes.size()), stdout);
            fputc('\n', stdout);
            fflush(stdout);
            QCoreApplication::exit(0);
            return;
        }
        if (root->property("pendingClose").toBool())
            QCoreApplication::quit();
    });
    timer->start();

    if (strcmp(mode, "--progress") == 0) {
        auto *n = new QSocketNotifier(fileno(stdin), QSocketNotifier::Read, &app);
        QObject::connect(n, &QSocketNotifier::activated, root, [root]() {
            char buf[4096];
            if (!fgets(buf, sizeof(buf), stdin)) {
                QCoreApplication::exit(0);
                return;
            }
            QMetaObject::invokeMethod(root, "applyLine", Q_ARG(QString, QString::fromUtf8(buf).trimmed()));
        });
    }

    return app.exec();
}

#ifndef KIRIGAMI_AS_LIBRARY
int main(int argc, char **argv)
{
    return kirigami_ui_run(argc, argv);
}
#endif
