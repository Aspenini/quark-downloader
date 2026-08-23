// Quark Downloader — Qt Quick helper. Dynamically links system Qt 6 and
// picks up platform themes such as CuteCosmic through Qt itself.
#include <QGuiApplication>
#include <QStyleHints>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QSocketNotifier>
#include <QFile>
#include <QDir>
#include <QByteArray>
#include <QList>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonArray>
#include <QTimer>
#include <cerrno>
#include <cstdio>
#include <cstring>
#include <fcntl.h>
#include <unistd.h>

static QString qmlDir()
{
    const QString fromEnv = qEnvironmentVariable("QUARK_QT_QML");
    if (!fromEnv.isEmpty() && QDir(fromEnv).exists())
        return fromEnv;
    const QString sibling = QCoreApplication::applicationDirPath() + "/qml";
    if (QDir(sibling).exists())
        return sibling;
#ifdef QUARK_QT_QML
    {
        const QString baked = QString::fromUtf8(QUARK_QT_QML);
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
        {"open_output_dir", b("openOutputDir")},
        {"gui_theme", s("theme")},
        {"strip_video_ids", b("stripIds")},
        {"sanitize_filenames", b("sanitize")},
        {"filename_spaces", s("spaces")},
        {"playlist_folders", b("playlistFolders")},
    };
}

extern "C" int qt_ui_run(int argc, char **argv)
{
    if (argc < 2) {
        fprintf(stderr, "usage: --session|--progress|--message\n");
        return 2;
    }
    // Snapshot before QGuiApplication. Qt treats `--session <id>` as its own
    // flag and would otherwise swallow the default output dir; the next leftover
    // argument is yt-dlp's "path", which then became a folder named "path".
    QList<QByteArray> rawArgs;
    rawArgs.reserve(argc);
    for (int i = 0; i < argc; ++i)
        rawArgs.append(QByteArray(argv[i]));

    const QByteArray modeBytes = rawArgs.at(1);
    const char *mode = modeBytes.constData();
    const bool progressMode = strcmp(mode, "--progress") == 0;
    // HelperProgress argv: --progress <unused> <theme>
    const QByteArray progressTheme = (progressMode && rawArgs.size() > 3) ? rawArgs.at(3) : QByteArray();

    int qtArgc = 1;
    char *qtArgv[] = { rawArgs[0].data(), nullptr };
    QGuiApplication app(qtArgc, qtArgv);

    QQmlApplicationEngine engine;
    QQmlContext *ctx = engine.rootContext();
    ctx->setContextProperty("quarkVersion", qEnvironmentVariable("QUARK_VERSION"));
    ctx->setContextProperty("quarkMode", QString::fromUtf8(mode));

    auto arg = [&](int i, const QString &fb) -> QString {
        const int idx = i + 2;
        return (idx < rawArgs.size()) ? QString::fromLocal8Bit(rawArgs.at(idx)) : fb;
    };
    auto barg = [&](int i, bool fb) {
        const int idx = i + 2;
        if (idx >= rawArgs.size())
            return fb;
        const QString v = QString::fromLocal8Bit(rawArgs.at(idx)).toLower();
        return v == "1" || v == "true" || v == "yes" || v == "on";
    };

    const QString defaultDir = arg(0, QDir::homePath() + "/Downloads");
    ctx->setContextProperty("defaultDir", defaultDir);
    ctx->setContextProperty("downloadDir", arg(1, QStringLiteral("~/Downloads")));
    ctx->setContextProperty("guiMode", arg(4, QStringLiteral("progress")));
    ctx->setContextProperty("logs", barg(5, true));
    ctx->setContextProperty("theme", arg(6, QStringLiteral("system")));
    if (progressMode && !progressTheme.isEmpty())
        applyColorScheme(QString::fromUtf8(progressTheme));
    else
        applyColorScheme(ctx->contextProperty("theme").toString());
    ctx->setContextProperty("stripIds", barg(7, true));
    ctx->setContextProperty("sanitize", barg(8, true));
    ctx->setContextProperty("spaces", arg(9, QStringLiteral("keep")));
    ctx->setContextProperty("playlistFolders", barg(10, true));
    ctx->setContextProperty("openOutputDir", barg(11, false));
    ctx->setContextProperty("outputDir", defaultDir);

    const QString qmlName = progressMode
        ? QStringLiteral("Progress.qml")
        : strcmp(mode, "--message") == 0
            ? QStringLiteral("Message.qml")
            : QStringLiteral("Session.qml");
    if (strcmp(mode, "--message") == 0) {
        ctx->setContextProperty("msgKind", rawArgs.size() > 2 ? QString::fromLocal8Bit(rawArgs.at(2)) : QStringLiteral("ok"));
        ctx->setContextProperty("msgTitle", rawArgs.size() > 3 ? QString::fromLocal8Bit(rawArgs.at(3)) : QStringLiteral("Quark Downloader"));
        QString body;
        for (int i = 4; i < rawArgs.size(); ++i) {
            if (i > 4)
                body += ' ';
            body += QString::fromLocal8Bit(rawArgs.at(i));
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

    if (progressMode) {
        const int fd = fileno(stdin);
        const int flags = fcntl(fd, F_GETFL, 0);
        if (flags >= 0)
            fcntl(fd, F_SETFL, flags | O_NONBLOCK);
    }

    auto applyProgressLine = [root](const QString &line) {
        if (line.isEmpty())
            return;
        const int tab = line.indexOf(QLatin1Char('\t'));
        const QString kind = tab < 0 ? line : line.left(tab);
        const QString rest = tab < 0 ? QString() : line.mid(tab + 1);
        if (kind == QLatin1String("PROGRESS"))
            root->setProperty("fraction", rest.toDouble() / 100.0);
        else if (kind == QLatin1String("STATUS"))
            root->setProperty("statusText", rest);
        else if (kind == QLatin1String("ETA"))
            root->setProperty("etaText", rest);
        else if (kind == QLatin1String("QUEUE"))
            root->setProperty("queueText", rest);
        else if (kind == QLatin1String("DONE"))
            QCoreApplication::exit(rest.toInt());
    };

    QByteArray stdinPending;
    auto drainStdin = [&stdinPending, progressMode, applyProgressLine]() -> bool {
        if (!progressMode)
            return false;
        char buf[4096];
        for (;;) {
            const ssize_t n = ::read(fileno(stdin), buf, sizeof(buf));
            if (n < 0)
                return errno != EAGAIN && errno != EWOULDBLOCK;
            if (n == 0)
                return true;
            stdinPending.append(buf, int(n));
            int nl;
            while ((nl = stdinPending.indexOf('\n')) >= 0) {
                const QString line = QString::fromUtf8(stdinPending.left(nl)).trimmed();
                stdinPending.remove(0, nl + 1);
                applyProgressLine(line);
            }
        }
    };

    // QML writes pendingSubmit; a typed QTimer slot forwards it to stdout.
    // The same tick drains progress stdin so pipe data is not stuck in stdio.
    auto *timer = new QTimer(&app);
    timer->setInterval(20);
    QObject::connect(timer, &QTimer::timeout, &app, [root, drainStdin]() {
        if (drainStdin()) {
            QCoreApplication::exit(0);
            return;
        }
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
    });
    timer->start();

    if (progressMode) {
        auto *n = new QSocketNotifier(fileno(stdin), QSocketNotifier::Read, &app);
        QObject::connect(n, &QSocketNotifier::activated, root, [drainStdin]() {
            if (drainStdin())
                QCoreApplication::exit(0);
        });
    }

    return app.exec();
}

#ifndef QUARK_QT_AS_LIBRARY
int main(int argc, char **argv)
{
    return qt_ui_run(argc, argv);
}
#endif
