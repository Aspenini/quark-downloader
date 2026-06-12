// Quark Downloader - native macOS UI helper (spawned by quark-downloader-gui).
// Speaks the same argv/stdout/stdin protocol as quark-downloader-gui.tcl.
import AppKit

func appWindowTitle() -> String {
    let version = ProcessInfo.processInfo.environment["QUARK_VERSION"] ?? ""
    return version.isEmpty ? "Quark Downloader" : "Quark Downloader \(version)"
}

func appSettingsWindowTitle() -> String {
    return "\(appWindowTitle()) Settings"
}

func normalizeTheme(_ value: String) -> String {
    return value.lowercased() == "dark" ? "dark" : "light"
}

func applyTheme(_ theme: String) {
    NSApp.appearance = NSAppearance(named: normalizeTheme(theme) == "dark" ? .darkAqua : .aqua)
}

func emitLines(_ lines: [String]) {
    let text = lines.joined(separator: "\n") + "\n"
    FileHandle.standardOutput.write(text.data(using: .utf8)!)
}

// Without a main menu, standard edit shortcuts (Cmd+V etc.) do not work.
func installMainMenu() {
    let mainMenu = NSMenu()

    let appItem = NSMenuItem()
    mainMenu.addItem(appItem)
    let appMenu = NSMenu()
    appMenu.addItem(
        withTitle: "Quit Quark Downloader",
        action: #selector(NSApplication.terminate(_:)),
        keyEquivalent: "q"
    )
    appItem.submenu = appMenu

    let editItem = NSMenuItem()
    mainMenu.addItem(editItem)
    let editMenu = NSMenu(title: "Edit")
    editMenu.addItem(withTitle: "Undo", action: Selector(("undo:")), keyEquivalent: "z")
    editMenu.addItem(withTitle: "Redo", action: Selector(("redo:")), keyEquivalent: "Z")
    editMenu.addItem(NSMenuItem.separator())
    editMenu.addItem(withTitle: "Cut", action: #selector(NSText.cut(_:)), keyEquivalent: "x")
    editMenu.addItem(withTitle: "Copy", action: #selector(NSText.copy(_:)), keyEquivalent: "c")
    editMenu.addItem(withTitle: "Paste", action: #selector(NSText.paste(_:)), keyEquivalent: "v")
    editMenu.addItem(
        withTitle: "Select All",
        action: #selector(NSText.selectAll(_:)),
        keyEquivalent: "a"
    )
    editItem.submenu = editMenu

    NSApp.mainMenu = mainMenu
}

let arguments = Array(CommandLine.arguments.dropFirst())

let app = NSApplication.shared
app.setActivationPolicy(.regular)
installMainMenu()

// Retained for the lifetime of NSApp.run().
var controllerHolder: AnyObject?

switch arguments.first {
case "--message":
    guard arguments.count >= 4 else {
        FileHandle.standardError.write(
            "usage: --message <ok|error> <title> <body>\n".data(using: .utf8)!
        )
        exit(2)
    }
    runMessageAlert(
        kind: arguments[1],
        title: arguments[2],
        body: arguments[3...].joined(separator: " ")
    )
case "--session":
    let session = SessionController(arguments: Array(arguments.dropFirst()))
    applyTheme(session.theme)
    controllerHolder = session
    session.show()
    app.activate(ignoringOtherApps: true)
    app.run()
case "--progress":
    var theme = "light"
    for argument in arguments.dropFirst() {
        let lowered = argument.lowercased()
        if lowered == "light" || lowered == "dark" {
            theme = lowered
        }
        // The other optional argument (logs dir) is unused, as in the Tk UI.
    }
    applyTheme(theme)
    let progress = ProgressController()
    controllerHolder = progress
    progress.show()
    app.activate(ignoringOtherApps: true)
    app.run()
default:
    FileHandle.standardError.write(
        "usage: --session ... | --progress ... | --message ...\n".data(using: .utf8)!
    )
    exit(2)
}

_ = controllerHolder
