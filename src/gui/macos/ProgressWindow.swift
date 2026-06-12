import AppKit

final class ProgressController: NSObject, NSWindowDelegate {
    let etaUpdateInterval: TimeInterval = 1.5

    var window: NSWindow!
    let statusLabel = NSTextField(labelWithString: "Starting download...")
    let queueLabel = NSTextField(labelWithString: "")
    let bar = NSProgressIndicator()
    let etaLabel = NSTextField(labelWithString: "Time left: estimating...")

    var eta = ""
    var lastEtaUpdate: TimeInterval = 0
    var pendingEtaUpdate: DispatchWorkItem?
    var finished = false
    var keyMonitor: Any?

    func show() {
        window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 420, height: 130),
            styleMask: [.titled, .miniaturizable],
            backing: .buffered,
            defer: false
        )
        window.delegate = self
        window.isReleasedWhenClosed = false
        updateTitle()

        statusLabel.lineBreakMode = .byTruncatingTail
        queueLabel.textColor = .secondaryLabelColor
        queueLabel.lineBreakMode = .byTruncatingTail
        etaLabel.lineBreakMode = .byTruncatingTail
        bar.isIndeterminate = false
        bar.minValue = 0
        bar.maxValue = 100
        bar.doubleValue = 0

        let stack = NSStackView(views: [statusLabel, queueLabel, bar, etaLabel])
        stack.orientation = .vertical
        stack.alignment = .leading
        stack.spacing = 8
        stack.translatesAutoresizingMaskIntoConstraints = false

        let content = NSView()
        content.addSubview(stack)
        NSLayoutConstraint.activate([
            stack.topAnchor.constraint(equalTo: content.topAnchor, constant: 14),
            stack.bottomAnchor.constraint(equalTo: content.bottomAnchor, constant: -14),
            stack.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 14),
            stack.trailingAnchor.constraint(equalTo: content.trailingAnchor, constant: -14),
            content.widthAnchor.constraint(equalToConstant: 420),
            bar.widthAnchor.constraint(equalTo: stack.widthAnchor),
            statusLabel.widthAnchor.constraint(equalTo: stack.widthAnchor),
            queueLabel.widthAnchor.constraint(equalTo: stack.widthAnchor),
        ])

        window.contentView = content
        content.layoutSubtreeIfNeeded()
        window.setContentSize(content.fittingSize)
        window.center()
        window.makeKeyAndOrderFront(nil)

        keyMonitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { event in
            if event.keyCode == 53 { // Escape
                exit(1)
            }
            return event
        }

        startReadingStdin()
    }

    private func startReadingStdin() {
        Thread.detachNewThread { [weak self] in
            while let line = readLine(strippingNewline: true) {
                DispatchQueue.main.async {
                    self?.apply(line: line)
                }
            }
            DispatchQueue.main.async {
                if self?.finished != true {
                    exit(1)
                }
            }
        }
    }

    private func apply(line: String) {
        guard !finished else { return }

        let parts = line.split(separator: "\t", maxSplits: 1, omittingEmptySubsequences: false)
        let kind = parts.first.map(String.init) ?? ""
        let payload = parts.count > 1 ? String(parts[1]) : ""

        switch kind {
        case "PROGRESS":
            if let value = Double(payload) {
                bar.doubleValue = min(max(value, 0), 100)
            }
        case "ETA":
            eta = payload
            scheduleEtaDisplayUpdate()
        case "STATUS":
            statusLabel.stringValue = payload
        case "QUEUE":
            queueLabel.stringValue = payload
        case "DONE":
            finished = true
            pendingEtaUpdate?.cancel()
            exit(Int32(payload) ?? 1)
        default:
            break
        }
    }

    private func scheduleEtaDisplayUpdate() {
        let now = Date().timeIntervalSince1970
        if lastEtaUpdate == 0 || now - lastEtaUpdate >= etaUpdateInterval {
            pendingEtaUpdate?.cancel()
            pendingEtaUpdate = nil
            applyEtaDisplayUpdate()
        } else if pendingEtaUpdate == nil {
            let delay = etaUpdateInterval - (now - lastEtaUpdate)
            let work = DispatchWorkItem { [weak self] in
                self?.pendingEtaUpdate = nil
                self?.applyEtaDisplayUpdate()
            }
            pendingEtaUpdate = work
            DispatchQueue.main.asyncAfter(deadline: .now() + delay, execute: work)
        }
    }

    private func applyEtaDisplayUpdate() {
        etaLabel.stringValue = eta.isEmpty ? "Time left: estimating..." : "Time left: \(eta) left"
        updateTitle()
        lastEtaUpdate = Date().timeIntervalSince1970
    }

    private func updateTitle() {
        if eta.isEmpty {
            window.title = "\(appWindowTitle()) - estimating..."
        } else {
            window.title = "\(appWindowTitle()) - \(eta) left"
        }
    }
}
