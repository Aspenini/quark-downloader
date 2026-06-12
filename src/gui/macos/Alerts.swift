import AppKit

func runMessageAlert(kind: String, title: String, body: String) -> Never {
    let alert = NSAlert()
    alert.messageText = title
    alert.informativeText = body
    alert.alertStyle = kind == "error" ? .critical : .informational
    alert.addButton(withTitle: "OK")
    NSApp.activate(ignoringOtherApps: true)
    alert.runModal()
    exit(0)
}

func showModalError(_ message: String, window: NSWindow?) {
    let alert = NSAlert()
    alert.messageText = "Quark Downloader"
    alert.informativeText = message
    alert.alertStyle = .critical
    alert.addButton(withTitle: "OK")
    if let window {
        alert.beginSheetModal(for: window)
    } else {
        alert.runModal()
    }
}
