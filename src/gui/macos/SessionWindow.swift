import AppKit

final class SessionController: NSObject, NSWindowDelegate, NSTableViewDataSource, NSTableViewDelegate {
    let audioFormats = ["original", "mp3", "m4a", "flac", "wav", "opus", "vorbis"]
    let videoFormats = ["original", "mp4", "mkv", "webm"]
    let spacesValues = ["keep", "underscore", "dash", "remove"]
    let modeValues = ["progress", "external_cli"]
    let themeValues = ["light", "dark"]

    // Session state; mirrors the variables the Tcl UI keeps.
    // yt-dlp / ffmpeg are always PATH via Homebrew — no source picker.
    var defaultDir: String
    var downloadDir: String
    var guiMode: String
    var logs: Bool
    var theme: String
    var stripIds: Bool
    var sanitize: Bool
    var spaces: String
    var playlistFolders: Bool
    var settingsSaved = false
    var queue: [String] = []
    var updateCheckRunning = false

    var window: NSWindow!
    var mainContainer: NSView!
    var settingsContainer: NSView!

    // Main view controls
    let urlField = NSTextField()
    let queueTable = NSTableView()
    let outputField = NSTextField()
    var videoRadio: NSButton!
    var audioRadio: NSButton!
    let formatPopup = NSPopUpButton()

    // Settings controls
    let settingsDirField = NSTextField()
    let themePopup = NSPopUpButton()
    var stripCheck: NSButton!
    var sanitizeCheck: NSButton!
    let spacesPopup = NSPopUpButton()
    var playlistCheck: NSButton!
    let modePopup = NSPopUpButton()
    var logsCheck: NSButton!
    var updatesButton: NSButton!

    init(arguments: [String]) {
        func arg(_ index: Int, _ fallback: String) -> String {
            return index < arguments.count ? arguments[index] : fallback
        }
        func boolArg(_ index: Int, _ fallback: Bool) -> Bool {
            guard index < arguments.count else { return fallback }
            return ["true", "1", "yes", "on"].contains(arguments[index].lowercased())
        }

        defaultDir = normalizedPath(arg(0, NSString(string: "~/Downloads").expandingTildeInPath))
        downloadDir = arg(1, "~/Downloads")
        // arg 2 = yt_dlp, arg 3 = ffmpeg — ignored (PATH / Homebrew only)
        guiMode = arg(4, "progress")
        logs = boolArg(5, true)
        theme = normalizeTheme(arg(6, "light"))
        stripIds = boolArg(7, true)
        sanitize = boolArg(8, true)
        spaces = arg(9, "keep")
        playlistFolders = boolArg(10, true)
        super.init()
    }

    func show() {
        window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 480, height: 400),
            styleMask: [.titled, .closable, .miniaturizable],
            backing: .buffered,
            defer: false
        )
        window.delegate = self
        window.isReleasedWhenClosed = false

        mainContainer = buildMainView()
        settingsContainer = buildSettingsView()
        outputField.stringValue = defaultDir

        showMain()
        window.center()
        window.makeKeyAndOrderFront(nil)
    }

    // MARK: - View swapping

    func showMain() {
        window.title = appWindowTitle()
        swapContent(to: mainContainer)
        window.makeFirstResponder(urlField)
    }

    func showSettings() {
        window.title = appSettingsWindowTitle()
        populateSettingsFields()
        swapContent(to: settingsContainer)
        window.makeFirstResponder(settingsDirField)
    }

    private func swapContent(to view: NSView) {
        window.contentView = view
        view.layoutSubtreeIfNeeded()
        window.setContentSize(view.fittingSize)
    }

    // MARK: - Main view

    private func buildMainView() -> NSView {
        let urlLabel = NSTextField(labelWithString: "Video or playlist URL:")
        urlField.placeholderString = "https://..."
        urlField.target = self
        urlField.action = #selector(addUrl)
        let addButton = NSButton(title: "Add", target: self, action: #selector(addUrl))
        let pasteButton = NSButton(title: "Paste", target: self, action: #selector(pasteUrls))
        let urlRow = hStack([urlField, addButton, pasteButton])
        expand(urlField)

        let queueLabel = NSTextField(labelWithString: "Queue:")
        let removeButton = NSButton(title: "Remove", target: self, action: #selector(removeSelected))
        let queueHeader = NSStackView()
        queueHeader.orientation = .horizontal
        queueHeader.addView(queueLabel, in: .leading)
        queueHeader.addView(removeButton, in: .trailing)

        let column = NSTableColumn(identifier: NSUserInterfaceItemIdentifier("url"))
        queueTable.addTableColumn(column)
        queueTable.headerView = nil
        queueTable.dataSource = self
        queueTable.delegate = self
        queueTable.allowsMultipleSelection = true
        queueTable.columnAutoresizingStyle = .uniformColumnAutoresizingStyle
        queueTable.registerForDraggedTypes([.string, .URL, .fileURL])
        let queueScroll = NSScrollView()
        queueScroll.documentView = queueTable
        queueScroll.hasVerticalScroller = true
        queueScroll.borderType = .bezelBorder
        queueScroll.heightAnchor.constraint(equalToConstant: 96).isActive = true

        videoRadio = NSButton(radioButtonWithTitle: "Video", target: self, action: #selector(typeChanged))
        audioRadio = NSButton(radioButtonWithTitle: "Audio", target: self, action: #selector(typeChanged))
        videoRadio.state = .on
        let typeRow = hStack([videoRadio, audioRadio])

        let formatLabel = NSTextField(labelWithString: "Format:")
        formatPopup.addItems(withTitles: videoFormats)
        let formatRow = hStack([formatLabel, formatPopup])

        let outputLabel = NSTextField(labelWithString: "Output folder:")
        let browseButton = NSButton(title: "Browse…", target: self, action: #selector(browseOutput))
        let outputRow = hStack([outputField, browseButton])
        expand(outputField)

        let settingsButton = NSButton(title: "⚙", target: self, action: #selector(openSettings))
        settingsButton.font = NSFont.systemFont(ofSize: 14)
        let downloadButton = NSButton(title: "Download", target: self, action: #selector(startDownload))
        downloadButton.keyEquivalent = "\r"
        let closeButton = NSButton(title: "Close", target: self, action: #selector(cancelSession))
        closeButton.keyEquivalent = "\u{1b}"
        let buttonRow = NSStackView()
        buttonRow.orientation = .horizontal
        buttonRow.addView(settingsButton, in: .leading)
        buttonRow.addView(downloadButton, in: .trailing)
        buttonRow.addView(closeButton, in: .trailing)

        return container(rows: [
            urlLabel, urlRow, queueHeader, queueScroll, typeRow,
            formatLabel, formatRow, outputLabel, outputRow, buttonRow,
        ], fullWidth: [urlRow, queueHeader, queueScroll, outputRow, buttonRow])
    }

    // MARK: - Settings view

    private func buildSettingsView() -> NSView {
        let dirLabel = NSTextField(labelWithString: "Default download folder:")
        let dirBrowse = NSButton(title: "Browse…", target: self, action: #selector(browseSettingsDir))
        let dirRow = hStack([settingsDirField, dirBrowse])
        expand(settingsDirField)
        themePopup.addItems(withTitles: themeValues)
        let themeRow = hStack([NSTextField(labelWithString: "Theme:"), themePopup])
        let generalBox = box("General", rows: [dirLabel, dirRow, themeRow], fullWidth: [dirRow])

        stripCheck = NSButton(checkboxWithTitle: "Remove trailing video ID from filenames", target: nil, action: nil)
        sanitizeCheck = NSButton(checkboxWithTitle: "Sanitize filenames (ASCII-safe)", target: nil, action: nil)
        spacesPopup.addItems(withTitles: spacesValues)
        let spacesRow = hStack([NSTextField(labelWithString: "Spaces in filenames:"), spacesPopup])
        playlistCheck = NSButton(checkboxWithTitle: "Put playlists in their own folder", target: nil, action: nil)
        let namingBox = box(
            "Download Naming",
            rows: [stripCheck, sanitizeCheck, spacesRow, playlistCheck],
            fullWidth: []
        )

        modePopup.addItems(withTitles: modeValues)
        let modeRow = hStack([NSTextField(labelWithString: "Download window:"), modePopup])
        logsCheck = NSButton(checkboxWithTitle: "Create download logs", target: nil, action: nil)
        let downloadsBox = box("Downloads", rows: [modeRow, logsCheck], fullWidth: [])

        updatesButton = NSButton(title: "Check for updates…", target: self, action: #selector(checkUpdates))
        let saveButton = NSButton(title: "Save", target: self, action: #selector(saveSettings))
        saveButton.keyEquivalent = "\r"
        let cancelButton = NSButton(title: "Cancel", target: self, action: #selector(closeSettings))
        cancelButton.keyEquivalent = "\u{1b}"
        let buttonRow = NSStackView()
        buttonRow.orientation = .horizontal
        buttonRow.addView(updatesButton, in: .leading)
        buttonRow.addView(saveButton, in: .trailing)
        buttonRow.addView(cancelButton, in: .trailing)

        return container(rows: [
            generalBox, namingBox, downloadsBox, buttonRow,
        ], fullWidth: [generalBox, namingBox, downloadsBox, buttonRow])
    }

    private func populateSettingsFields() {
        settingsDirField.stringValue = downloadDir
        select(themePopup, theme, from: themeValues)
        stripCheck.state = stripIds ? .on : .off
        sanitizeCheck.state = sanitize ? .on : .off
        select(spacesPopup, spaces, from: spacesValues)
        playlistCheck.state = playlistFolders ? .on : .off
        select(modePopup, guiMode, from: modeValues)
        logsCheck.state = logs ? .on : .off
    }

    // MARK: - Layout helpers

    private func hStack(_ views: [NSView]) -> NSStackView {
        let stack = NSStackView(views: views)
        stack.orientation = .horizontal
        stack.spacing = 8
        return stack
    }

    private func expand(_ view: NSView) {
        view.setContentHuggingPriority(.init(1), for: .horizontal)
        if let field = view as? NSTextField {
            field.widthAnchor.constraint(greaterThanOrEqualToConstant: 200).isActive = true
        }
    }

    private func container(rows: [NSView], fullWidth: [NSView]) -> NSView {
        let stack = NSStackView(views: rows)
        stack.orientation = .vertical
        stack.alignment = .leading
        stack.spacing = 10
        stack.translatesAutoresizingMaskIntoConstraints = false

        let containerView = NSView()
        containerView.addSubview(stack)
        NSLayoutConstraint.activate([
            stack.topAnchor.constraint(equalTo: containerView.topAnchor, constant: 14),
            stack.bottomAnchor.constraint(equalTo: containerView.bottomAnchor, constant: -14),
            stack.leadingAnchor.constraint(equalTo: containerView.leadingAnchor, constant: 14),
            stack.trailingAnchor.constraint(equalTo: containerView.trailingAnchor, constant: -14),
            containerView.widthAnchor.constraint(equalToConstant: 480),
        ])
        for view in fullWidth {
            view.widthAnchor.constraint(equalTo: stack.widthAnchor).isActive = true
        }
        return containerView
    }

    private func box(_ title: String, rows: [NSView], fullWidth: [NSView]) -> NSBox {
        let stack = NSStackView(views: rows)
        stack.orientation = .vertical
        stack.alignment = .leading
        stack.spacing = 8
        stack.translatesAutoresizingMaskIntoConstraints = false

        let boxView = NSBox()
        boxView.title = title
        let content = NSView()
        content.addSubview(stack)
        NSLayoutConstraint.activate([
            stack.topAnchor.constraint(equalTo: content.topAnchor, constant: 6),
            stack.bottomAnchor.constraint(equalTo: content.bottomAnchor, constant: -6),
            stack.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 8),
            stack.trailingAnchor.constraint(equalTo: content.trailingAnchor, constant: -8),
        ])
        for view in fullWidth {
            view.widthAnchor.constraint(equalTo: stack.widthAnchor).isActive = true
        }
        boxView.contentView = content
        return boxView
    }

    private func select(_ popup: NSPopUpButton, _ value: String, from values: [String]) {
        let index = values.firstIndex(of: value) ?? 0
        popup.selectItem(at: index)
    }

    // MARK: - Queue table

    func numberOfRows(in tableView: NSTableView) -> Int {
        return queue.count
    }

    func tableView(_ tableView: NSTableView, viewFor tableColumn: NSTableColumn?, row: Int) -> NSView? {
        let identifier = NSUserInterfaceItemIdentifier("urlCell")
        let label: NSTextField
        if let reused = tableView.makeView(withIdentifier: identifier, owner: nil) as? NSTextField {
            label = reused
        } else {
            label = NSTextField(labelWithString: "")
            label.identifier = identifier
            label.lineBreakMode = .byTruncatingMiddle
        }
        label.stringValue = queue[row]
        return label
    }

    // MARK: - Actions

    @objc func addUrl() {
        enqueueUrl(urlField.stringValue)
        urlField.stringValue = ""
        window.makeFirstResponder(urlField)
    }

    @objc func pasteUrls() {
        guard let clip = NSPasteboard.general.string(forType: .string) else { return }
        clip.split(whereSeparator: \.isNewline).forEach { enqueueUrl(String($0)) }
    }

    private func enqueueUrl(_ raw: String) {
        let url = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !url.isEmpty else { return }
        if !queue.contains(url) {
            queue.append(url)
            queueTable.reloadData()
        }
    }

    // Drag-and-drop URLs onto the queue table
    func tableView(_ tableView: NSTableView, validateDrop info: NSDraggingInfo,
                   proposedRow row: Int, proposedDropOperation dropOperation: NSTableView.DropOperation) -> NSDragOperation {
        tableView.setDropRow(-1, dropOperation: .on)
        return .copy
    }

    func tableView(_ tableView: NSTableView, acceptDrop info: NSDraggingInfo,
                   row: Int, dropOperation: NSTableView.DropOperation) -> Bool {
        let pb = info.draggingPasteboard
        if let str = pb.string(forType: .string) {
            str.split(whereSeparator: \.isNewline).forEach { enqueueUrl(String($0)) }
            return true
        }
        if let urls = pb.readObjects(forClasses: [NSURL.self], options: nil) as? [URL] {
            urls.forEach { enqueueUrl($0.absoluteString) }
            return true
        }
        return false
    }

    @objc func removeSelected() {
        let selected = queueTable.selectedRowIndexes
        guard !selected.isEmpty else { return }
        queue = queue.enumerated().filter { !selected.contains($0.offset) }.map { $0.element }
        queueTable.reloadData()
    }

    @objc func typeChanged() {
        let formats = audioRadio.state == .on ? audioFormats : videoFormats
        formatPopup.removeAllItems()
        formatPopup.addItems(withTitles: formats)
        formatPopup.selectItem(at: 0)
    }

    @objc func browseOutput() {
        browseDirectory(
            initial: outputField.stringValue.isEmpty ? defaultDir : outputField.stringValue,
            title: "Select output folder"
        ) { [weak self] path in
            self?.outputField.stringValue = path
        }
    }

    @objc func browseSettingsDir() {
        browseDirectory(
            initial: settingsDirField.stringValue.isEmpty
                ? NSString(string: "~/Downloads").expandingTildeInPath
                : settingsDirField.stringValue,
            title: "Select default download folder"
        ) { [weak self] path in
            self?.settingsDirField.stringValue = path
        }
    }

    private func browseDirectory(initial: String, title: String, onPick: @escaping (String) -> Void) {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.canCreateDirectories = true
        panel.allowsMultipleSelection = false
        panel.message = title
        panel.directoryURL = URL(fileURLWithPath: normalizedPath(initial), isDirectory: true)
        panel.beginSheetModal(for: window) { response in
            if response == .OK, let url = panel.url {
                onPick(url.path)
            }
        }
    }

    @objc func openSettings() {
        showSettings()
    }

    @objc func closeSettings() {
        showMain()
    }

    @objc func saveSettings() {
        let dir = settingsDirField.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !dir.isEmpty else {
            showModalError("Please choose a default download folder.", window: window)
            return
        }

        let previousDefault = defaultDir
        let normalizedDir = normalizedPath(dir)
        let currentOutput = outputField.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)

        downloadDir = dir
        theme = normalizeTheme(themePopup.titleOfSelectedItem ?? "light")
        stripIds = stripCheck.state == .on
        sanitize = sanitizeCheck.state == .on
        spaces = spacesPopup.titleOfSelectedItem ?? "keep"
        playlistFolders = playlistCheck.state == .on
        guiMode = modePopup.titleOfSelectedItem ?? "progress"
        logs = logsCheck.state == .on
        defaultDir = normalizedDir
        settingsSaved = true
        applyTheme(theme)

        if currentOutput.isEmpty || currentOutput == previousDefault {
            outputField.stringValue = normalizedDir
        }

        showMain()
    }

    @objc func checkUpdates() {
        guard !updateCheckRunning else { return }

        let helperDir = URL(fileURLWithPath: CommandLine.arguments[0])
            .resolvingSymlinksInPath()
            .deletingLastPathComponent()
        let gui = helperDir.appendingPathComponent("quark-downloader-gui")
        guard FileManager.default.isExecutableFile(atPath: gui.path) else {
            showModalError("quark-downloader-gui was not found next to the GUI helper.", window: window)
            return
        }

        updateCheckRunning = true
        updatesButton.title = "Checking…"
        updatesButton.isEnabled = false

        let process = Process()
        process.executableURL = gui
        process.arguments = ["--check-updates"]
        do {
            try process.run()
        } catch {
            resetUpdatesButton()
            showModalError("Could not check for updates:\n\(error.localizedDescription)", window: window)
            return
        }

        DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) { [weak self] in
            self?.resetUpdatesButton()
        }
    }

    private func resetUpdatesButton() {
        updateCheckRunning = false
        updatesButton.title = "Check for updates…"
        updatesButton.isEnabled = true
    }

    @objc func startDownload() {
        addUrl()

        guard !queue.isEmpty else {
            showModalError("Please enter at least one video or playlist URL.", window: window)
            return
        }

        let output = outputField.stringValue.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !output.isEmpty else {
            showModalError("Please choose an output folder.", window: window)
            return
        }

        let mediaType = audioRadio.state == .on ? "audio" : "video"
        let format = formatPopup.titleOfSelectedItem ?? "original"
        emitDownload(urls: queue, mediaType: mediaType, format: format, output: output)
    }

    @objc func cancelSession() {
        emitCancel()
    }

    func windowShouldClose(_ sender: NSWindow) -> Bool {
        emitCancel()
    }

    // MARK: - Protocol emission (JSON v1)

    private func settingsObject() -> [String: Any] {
        return [
            "download_dir": downloadDir,
            "yt_dlp": "path",
            "ffmpeg": "path",
            "gui_download_mode": guiMode,
            "download_logs": logs,
            "gui_theme": theme,
            "strip_video_ids": stripIds,
            "sanitize_filenames": sanitize,
            "filename_spaces": spaces,
            "playlist_folders": playlistFolders,
        ]
    }

    private func emitJSON(_ object: [String: Any]) -> Never {
        guard let data = try? JSONSerialization.data(withJSONObject: object, options: []),
              let text = String(data: data, encoding: .utf8) else {
            fputs("{}\n", stdout)
            exit(1)
        }
        fputs(text + "\n", stdout)
        fflush(stdout)
        exit(0)
    }

    private func emitDownload(urls: [String], mediaType: String, format: String, output: String) -> Never {
        var object: [String: Any] = [
            "v": 1,
            "action": "download",
            "urls": urls,
            "media_type": mediaType,
            "format": format,
            "output_dir": output,
        ]
        if settingsSaved {
            object["settings"] = settingsObject()
        }
        emitJSON(object)
    }

    private func emitCancel() -> Never {
        var object: [String: Any] = [
            "v": 1,
            "action": "cancel",
        ]
        if settingsSaved {
            object["settings"] = settingsObject()
        }
        emitJSON(object)
    }
}

func normalizedPath(_ path: String) -> String {
    let expanded = NSString(string: path).expandingTildeInPath
    return URL(fileURLWithPath: expanded).standardizedFileURL.path
}
