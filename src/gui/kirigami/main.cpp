// Quark Downloader — Kirigami helper. Dynamically links system Qt 6.
// Kirigami itself is a QML module provided by the distro.
#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QSocketNotifier>
#include <QFile>
#include <QDir>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonArray>
#include <cstdio>
#include <cstring>

static QString qmlDir()
{
    const QString sibling = QCoreApplication::applicationDirPath() + "/qml";
    if (QDir(sibling).exists())
        return sibling;
#ifdef QUARK_KIRIGAMI_QML
    return QString::fromUtf8(QUARK_KIRIGAMI_QML);
#else
    return QCoreApplication::applicationDirPath();
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

int main(int argc, char **argv)
{
    if (argc < 2) {
        fprintf(stderr, "usage: --session|--progress|--message\n");
        return 2;
    }
    const char *mode = argv[1];

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
    ctx->setContextProperty("theme", arg(6, QStringLiteral("light")));
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

    QObject::connect(root, SIGNAL(submit(QString)), &app, [&](const QString &json) {
        fwrite(json.toUtf8().constData(), 1, size_t(json.toUtf8().size()), stdout);
        fputc('\n', stdout);
        fflush(stdout);
        QCoreApplication::exit(0);
    });
    QObject::connect(root, SIGNAL(requestSettings()), &app, [ctx, root]() {
        const QJsonDocument doc(settingsFromCtx(ctx));
        QMetaObject::invokeMethod(root, "takeSettingsJson", Q_ARG(QVariant, QString::fromUtf8(doc.toJson(QJsonDocument::Compact))));
    });
    QObject::connect(root, SIGNAL(closed()), &app, &QCoreApplication::quit);

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
