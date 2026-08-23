#import "SessionWindow.h"
#import "QuarkMac.h"

#import <stdio.h>

static NSArray<NSString *> *AudioFormats(void) {
    return @[@"original", @"mp3", @"m4a", @"flac", @"wav", @"opus", @"vorbis"];
}

static NSArray<NSString *> *VideoFormats(void) {
    return @[@"original", @"mp4", @"mkv", @"webm"];
}

static NSArray<NSString *> *SpacesValues(void) {
    return @[@"keep", @"underscore", @"dash", @"remove"];
}

static NSArray<NSString *> *ModeValues(void) {
    return @[@"progress", @"external_cli"];
}

static NSArray<NSString *> *ThemeValues(void) {
    return @[@"system", @"light", @"dark"];
}

static NSStackView *QuarkHStack(NSArray<NSView *> *views) {
    NSStackView *stack = [NSStackView stackViewWithViews:views];
    stack.orientation = NSUserInterfaceLayoutOrientationHorizontal;
    stack.spacing = 8;
    return stack;
}

static void QuarkExpand(NSView *view) {
    [view setContentHuggingPriority:1.0 forOrientation:NSLayoutConstraintOrientationHorizontal];
    if ([view isKindOfClass:[NSTextField class]]) {
        [view.widthAnchor constraintGreaterThanOrEqualToConstant:200].active = YES;
    }
}

static void QuarkSelect(NSPopUpButton *popup, NSString *value, NSArray<NSString *> *values) {
    NSUInteger index = [values indexOfObject:value];
    [popup selectItemAtIndex:index == NSNotFound ? 0 : (NSInteger)index];
}

@interface SessionController ()
@property (nonatomic, copy, readwrite) NSString *theme;
@property (nonatomic, copy) NSString *defaultDir;
@property (nonatomic, copy) NSString *downloadDir;
@property (nonatomic, copy) NSString *guiMode;
@property (nonatomic) BOOL logs;
@property (nonatomic) BOOL stripIds;
@property (nonatomic) BOOL sanitize;
@property (nonatomic, copy) NSString *spaces;
@property (nonatomic) BOOL playlistFolders;
@property (nonatomic) BOOL openOutputDir;
@property (nonatomic) BOOL settingsSaved;
@property (nonatomic, strong) NSMutableArray<NSString *> *queue;
@property (nonatomic) BOOL updateCheckRunning;

@property (nonatomic, strong) NSWindow *window;
@property (nonatomic, strong) NSView *mainContainer;
@property (nonatomic, strong) NSView *settingsContainer;

@property (nonatomic, strong) NSTextField *urlField;
@property (nonatomic, strong) NSTableView *queueTable;
@property (nonatomic, strong) NSTextField *outputField;
@property (nonatomic, strong) NSButton *videoRadio;
@property (nonatomic, strong) NSButton *audioRadio;
@property (nonatomic, strong) NSPopUpButton *formatPopup;

@property (nonatomic, strong) NSTextField *settingsDirField;
@property (nonatomic, strong) NSPopUpButton *themePopup;
@property (nonatomic, strong) NSButton *stripCheck;
@property (nonatomic, strong) NSButton *sanitizeCheck;
@property (nonatomic, strong) NSPopUpButton *spacesPopup;
@property (nonatomic, strong) NSButton *playlistCheck;
@property (nonatomic, strong) NSPopUpButton *modePopup;
@property (nonatomic, strong) NSButton *logsCheck;
@property (nonatomic, strong) NSButton *openOutputCheck;
@property (nonatomic, strong) NSButton *updatesButton;
@end

@implementation SessionController

- (instancetype)initWithArguments:(NSArray<NSString *> *)arguments {
    self = [super init];
    if (self == nil) {
        return nil;
    }
    NSString * (^arg)(NSUInteger, NSString *) = ^(NSUInteger index, NSString *fallback) {
        return index < arguments.count ? arguments[index] : fallback;
    };
    BOOL (^boolArg)(NSUInteger, BOOL) = ^(NSUInteger index, BOOL fallback) {
        if (index >= arguments.count) {
            return fallback;
        }
        return QuarkParseBool(arguments[index], fallback);
    };

    NSString *homeDownloads = [@"~/Downloads" stringByExpandingTildeInPath];
    _defaultDir = [QuarkNormalizedPath(arg(0, homeDownloads)) copy];
    _downloadDir = [arg(1, @"~/Downloads") copy];
    // arg 2 = yt_dlp, arg 3 = ffmpeg — ignored (PATH / Homebrew only)
    _guiMode = [arg(4, @"progress") copy];
    _logs = boolArg(5, YES);
    _theme = [QuarkNormalizeTheme(arg(6, @"system")) copy];
    _stripIds = boolArg(7, YES);
    _sanitize = boolArg(8, YES);
    _spaces = [arg(9, @"keep") copy];
    _playlistFolders = boolArg(10, YES);
    _openOutputDir = boolArg(11, NO);
    _queue = [NSMutableArray array];
    return self;
}

- (void)show {
    self.window = [[NSWindow alloc]
        initWithContentRect:NSMakeRect(0, 0, 480, 400)
                  styleMask:(NSWindowStyleMaskTitled | NSWindowStyleMaskClosable | NSWindowStyleMaskMiniaturizable)
                    backing:NSBackingStoreBuffered
                      defer:NO];
    self.window.delegate = self;
    self.window.releasedWhenClosed = NO;

    self.mainContainer = [self buildMainView];
    self.settingsContainer = [self buildSettingsView];
    self.outputField.stringValue = self.defaultDir;

    [self showMain];
    [self.window center];
    [self.window makeKeyAndOrderFront:nil];
}

- (void)showMain {
    self.window.title = QuarkAppWindowTitle();
    [self swapContent:self.mainContainer];
    [self.window makeFirstResponder:self.urlField];
}

- (void)showSettings {
    self.window.title = QuarkAppSettingsWindowTitle();
    [self populateSettingsFields];
    [self swapContent:self.settingsContainer];
    [self.window makeFirstResponder:self.settingsDirField];
}

- (void)swapContent:(NSView *)view {
    self.window.contentView = view;
    [view layoutSubtreeIfNeeded];
    [self.window setContentSize:view.fittingSize];
}

- (NSView *)containerWithRows:(NSArray<NSView *> *)rows fullWidth:(NSArray<NSView *> *)fullWidth {
    NSStackView *stack = [NSStackView stackViewWithViews:rows];
    stack.orientation = NSUserInterfaceLayoutOrientationVertical;
    stack.alignment = NSLayoutAttributeLeading;
    stack.spacing = 10;
    stack.translatesAutoresizingMaskIntoConstraints = NO;

    NSView *containerView = [[NSView alloc] init];
    [containerView addSubview:stack];
    [NSLayoutConstraint activateConstraints:@[
        [stack.topAnchor constraintEqualToAnchor:containerView.topAnchor constant:14],
        [stack.bottomAnchor constraintEqualToAnchor:containerView.bottomAnchor constant:-14],
        [stack.leadingAnchor constraintEqualToAnchor:containerView.leadingAnchor constant:14],
        [stack.trailingAnchor constraintEqualToAnchor:containerView.trailingAnchor constant:-14],
        [containerView.widthAnchor constraintEqualToConstant:480],
    ]];
    for (NSView *view in fullWidth) {
        [view.widthAnchor constraintEqualToAnchor:stack.widthAnchor].active = YES;
    }
    return containerView;
}

- (NSBox *)boxWithTitle:(NSString *)title rows:(NSArray<NSView *> *)rows fullWidth:(NSArray<NSView *> *)fullWidth {
    NSStackView *stack = [NSStackView stackViewWithViews:rows];
    stack.orientation = NSUserInterfaceLayoutOrientationVertical;
    stack.alignment = NSLayoutAttributeLeading;
    stack.spacing = 8;
    stack.translatesAutoresizingMaskIntoConstraints = NO;

    NSBox *boxView = [[NSBox alloc] init];
    boxView.title = title;
    NSView *content = boxView.contentView;
    [content addSubview:stack];
    [NSLayoutConstraint activateConstraints:@[
        [stack.topAnchor constraintEqualToAnchor:content.topAnchor constant:6],
        [stack.bottomAnchor constraintEqualToAnchor:content.bottomAnchor constant:-6],
        [stack.leadingAnchor constraintEqualToAnchor:content.leadingAnchor constant:8],
        [stack.trailingAnchor constraintEqualToAnchor:content.trailingAnchor constant:-8],
    ]];
    for (NSView *view in fullWidth) {
        [view.widthAnchor constraintEqualToAnchor:stack.widthAnchor].active = YES;
    }
    return boxView;
}

- (NSView *)buildMainView {
    NSTextField *urlLabel = [NSTextField labelWithString:@"Video or playlist URL:"];
    self.urlField = [[NSTextField alloc] init];
    self.urlField.placeholderString = @"https://...";
    self.urlField.target = self;
    self.urlField.action = @selector(addUrl);
    NSButton *addButton = [NSButton buttonWithTitle:@"Add" target:self action:@selector(addUrl)];
    NSButton *pasteButton = [NSButton buttonWithTitle:@"Paste" target:self action:@selector(pasteUrls)];
    NSStackView *urlRow = QuarkHStack(@[self.urlField, addButton, pasteButton]);
    QuarkExpand(self.urlField);

    NSTextField *queueLabel = [NSTextField labelWithString:@"Queue:"];
    NSButton *removeButton = [NSButton buttonWithTitle:@"Remove" target:self action:@selector(removeSelected)];
    NSStackView *queueHeader = [[NSStackView alloc] init];
    queueHeader.orientation = NSUserInterfaceLayoutOrientationHorizontal;
    [queueHeader addView:queueLabel inGravity:NSStackViewGravityLeading];
    [queueHeader addView:removeButton inGravity:NSStackViewGravityTrailing];

    self.queueTable = [[NSTableView alloc] init];
    NSTableColumn *column = [[NSTableColumn alloc] initWithIdentifier:@"url"];
    [self.queueTable addTableColumn:column];
    self.queueTable.headerView = nil;
    self.queueTable.dataSource = self;
    self.queueTable.delegate = self;
    self.queueTable.allowsMultipleSelection = YES;
    self.queueTable.columnAutoresizingStyle = NSTableViewUniformColumnAutoresizingStyle;
    [self.queueTable registerForDraggedTypes:@[
        NSPasteboardTypeString, NSPasteboardTypeURL, NSPasteboardTypeFileURL
    ]];
    NSScrollView *queueScroll = [[NSScrollView alloc] init];
    queueScroll.documentView = self.queueTable;
    queueScroll.hasVerticalScroller = YES;
    queueScroll.borderType = NSBezelBorder;
    [queueScroll.heightAnchor constraintEqualToConstant:96].active = YES;

    self.videoRadio = [NSButton radioButtonWithTitle:@"Video" target:self action:@selector(typeChanged)];
    self.audioRadio = [NSButton radioButtonWithTitle:@"Audio" target:self action:@selector(typeChanged)];
    self.videoRadio.state = NSControlStateValueOn;
    NSStackView *typeRow = QuarkHStack(@[self.videoRadio, self.audioRadio]);

    NSTextField *formatLabel = [NSTextField labelWithString:@"Format:"];
    self.formatPopup = [[NSPopUpButton alloc] init];
    [self.formatPopup addItemsWithTitles:VideoFormats()];
    NSStackView *formatRow = QuarkHStack(@[self.formatPopup]);

    NSTextField *outputLabel = [NSTextField labelWithString:@"Output folder:"];
    self.outputField = [[NSTextField alloc] init];
    NSButton *browseButton = [NSButton buttonWithTitle:@"Browse…" target:self action:@selector(browseOutput)];
    NSStackView *outputRow = QuarkHStack(@[self.outputField, browseButton]);
    QuarkExpand(self.outputField);

    NSButton *settingsButton = [NSButton buttonWithTitle:@"⚙" target:self action:@selector(openSettings)];
    settingsButton.font = [NSFont systemFontOfSize:14];
    NSButton *downloadButton = [NSButton buttonWithTitle:@"Download" target:self action:@selector(startDownload)];
    downloadButton.keyEquivalent = @"\r";
    NSButton *closeButton = [NSButton buttonWithTitle:@"Close" target:self action:@selector(cancelSession)];
    closeButton.keyEquivalent = @"\033";
    NSStackView *buttonRow = [[NSStackView alloc] init];
    buttonRow.orientation = NSUserInterfaceLayoutOrientationHorizontal;
    [buttonRow addView:settingsButton inGravity:NSStackViewGravityLeading];
    [buttonRow addView:downloadButton inGravity:NSStackViewGravityTrailing];
    [buttonRow addView:closeButton inGravity:NSStackViewGravityTrailing];

    return [self containerWithRows:@[
        urlLabel, urlRow, queueHeader, queueScroll, typeRow, formatLabel, formatRow, outputLabel,
        outputRow, buttonRow
    ] fullWidth:@[urlRow, queueHeader, queueScroll, outputRow, buttonRow]];
}

- (NSView *)buildSettingsView {
    NSTextField *dirLabel = [NSTextField labelWithString:@"Default download folder:"];
    self.settingsDirField = [[NSTextField alloc] init];
    NSButton *dirBrowse = [NSButton buttonWithTitle:@"Browse…" target:self action:@selector(browseSettingsDir)];
    NSStackView *dirRow = QuarkHStack(@[self.settingsDirField, dirBrowse]);
    QuarkExpand(self.settingsDirField);
    self.themePopup = [[NSPopUpButton alloc] init];
    [self.themePopup addItemsWithTitles:ThemeValues()];
    NSStackView *themeRow = QuarkHStack(@[[NSTextField labelWithString:@"Theme:"], self.themePopup]);
    NSBox *generalBox = [self boxWithTitle:@"General" rows:@[dirLabel, dirRow, themeRow] fullWidth:@[dirRow]];

    self.stripCheck = [NSButton checkboxWithTitle:@"Remove trailing video ID from filenames"
                                           target:nil
                                           action:nil];
    self.sanitizeCheck = [NSButton checkboxWithTitle:@"Sanitize filenames (ASCII-safe)"
                                              target:nil
                                              action:nil];
    self.spacesPopup = [[NSPopUpButton alloc] init];
    [self.spacesPopup addItemsWithTitles:SpacesValues()];
    NSStackView *spacesRow =
        QuarkHStack(@[[NSTextField labelWithString:@"Spaces in filenames:"], self.spacesPopup]);
    self.playlistCheck = [NSButton checkboxWithTitle:@"Put playlists in their own folder"
                                              target:nil
                                              action:nil];
    NSBox *namingBox = [self boxWithTitle:@"Download Naming"
                                     rows:@[self.stripCheck, self.sanitizeCheck, spacesRow, self.playlistCheck]
                                fullWidth:@[]];

    self.modePopup = [[NSPopUpButton alloc] init];
    [self.modePopup addItemsWithTitles:ModeValues()];
    NSStackView *modeRow = QuarkHStack(@[[NSTextField labelWithString:@"Download window:"], self.modePopup]);
    self.logsCheck = [NSButton checkboxWithTitle:@"Create download logs" target:nil action:nil];
    self.openOutputCheck = [NSButton checkboxWithTitle:@"Open output folder when done" target:nil action:nil];
    NSBox *downloadsBox = [self boxWithTitle:@"Downloads"
                                        rows:@[modeRow, self.logsCheck, self.openOutputCheck]
                                   fullWidth:@[]];

    self.updatesButton = [NSButton buttonWithTitle:@"Check for updates…"
                                            target:self
                                            action:@selector(checkUpdates)];
    NSButton *resetButton = [NSButton buttonWithTitle:@"Reset to defaults"
                                               target:self
                                               action:@selector(resetSettings)];
    NSButton *saveButton = [NSButton buttonWithTitle:@"Save" target:self action:@selector(saveSettings)];
    saveButton.keyEquivalent = @"\r";
    NSButton *cancelButton = [NSButton buttonWithTitle:@"Cancel" target:self action:@selector(closeSettings)];
    cancelButton.keyEquivalent = @"\033";
    NSStackView *buttonRow = [[NSStackView alloc] init];
    buttonRow.orientation = NSUserInterfaceLayoutOrientationHorizontal;
    [buttonRow addView:self.updatesButton inGravity:NSStackViewGravityLeading];
    [buttonRow addView:resetButton inGravity:NSStackViewGravityLeading];
    [buttonRow addView:saveButton inGravity:NSStackViewGravityTrailing];
    [buttonRow addView:cancelButton inGravity:NSStackViewGravityTrailing];

    return [self containerWithRows:@[generalBox, namingBox, downloadsBox, buttonRow]
                         fullWidth:@[generalBox, namingBox, downloadsBox, buttonRow]];
}

- (void)populateSettingsFields {
    self.settingsDirField.stringValue = self.downloadDir;
    QuarkSelect(self.themePopup, self.theme, ThemeValues());
    self.stripCheck.state = self.stripIds ? NSControlStateValueOn : NSControlStateValueOff;
    self.sanitizeCheck.state = self.sanitize ? NSControlStateValueOn : NSControlStateValueOff;
    QuarkSelect(self.spacesPopup, self.spaces, SpacesValues());
    self.playlistCheck.state = self.playlistFolders ? NSControlStateValueOn : NSControlStateValueOff;
    QuarkSelect(self.modePopup, self.guiMode, ModeValues());
    self.logsCheck.state = self.logs ? NSControlStateValueOn : NSControlStateValueOff;
    self.openOutputCheck.state = self.openOutputDir ? NSControlStateValueOn : NSControlStateValueOff;
}

- (NSInteger)numberOfRowsInTableView:(NSTableView *)tableView {
    (void)tableView;
    return (NSInteger)self.queue.count;
}

- (NSView *)tableView:(NSTableView *)tableView
    viewForTableColumn:(NSTableColumn *)tableColumn
                   row:(NSInteger)row {
    (void)tableColumn;
    NSString *ident = @"urlCell";
    NSTextField *label = [tableView makeViewWithIdentifier:ident owner:nil];
    if (label == nil) {
        label = [NSTextField labelWithString:@""];
        label.identifier = ident;
        label.lineBreakMode = NSLineBreakByTruncatingMiddle;
    }
    label.stringValue = self.queue[(NSUInteger)row];
    return label;
}

- (NSDragOperation)tableView:(NSTableView *)tableView
                validateDrop:(id<NSDraggingInfo>)info
                 proposedRow:(NSInteger)row
       proposedDropOperation:(NSTableViewDropOperation)dropOperation {
    (void)info;
    (void)row;
    (void)dropOperation;
    [tableView setDropRow:-1 dropOperation:NSTableViewDropOn];
    return NSDragOperationCopy;
}

- (BOOL)tableView:(NSTableView *)tableView
       acceptDrop:(id<NSDraggingInfo>)info
              row:(NSInteger)row
    dropOperation:(NSTableViewDropOperation)dropOperation {
    (void)tableView;
    (void)row;
    (void)dropOperation;
    NSPasteboard *pb = info.draggingPasteboard;
    NSString *str = [pb stringForType:NSPasteboardTypeString];
    if (str != nil) {
        for (NSString *line in [str componentsSeparatedByCharactersInSet:NSCharacterSet.newlineCharacterSet]) {
            [self enqueueUrl:line];
        }
        return YES;
    }
    NSArray *urls = [pb readObjectsForClasses:@[[NSURL class]] options:nil];
    if (urls.count > 0) {
        for (NSURL *url in urls) {
            [self enqueueUrl:url.absoluteString];
        }
        return YES;
    }
    return NO;
}

- (void)addUrl {
    [self enqueueUrl:self.urlField.stringValue];
    self.urlField.stringValue = @"";
    [self.window makeFirstResponder:self.urlField];
}

- (void)pasteUrls {
    NSString *clip = [NSPasteboard.generalPasteboard stringForType:NSPasteboardTypeString];
    if (clip == nil) {
        return;
    }
    for (NSString *line in [clip componentsSeparatedByCharactersInSet:NSCharacterSet.newlineCharacterSet]) {
        [self enqueueUrl:line];
    }
}

- (void)enqueueUrl:(NSString *)raw {
    NSString *url = [raw stringByTrimmingCharactersInSet:NSCharacterSet.whitespaceAndNewlineCharacterSet];
    if (url.length == 0) {
        return;
    }
    if (![self.queue containsObject:url]) {
        [self.queue addObject:url];
        [self.queueTable reloadData];
    }
}

- (void)removeSelected {
    NSIndexSet *selected = self.queueTable.selectedRowIndexes;
    if (selected.count == 0) {
        return;
    }
    NSMutableArray<NSString *> *kept = [NSMutableArray array];
    [self.queue enumerateObjectsUsingBlock:^(NSString *url, NSUInteger idx, BOOL *stop) {
        (void)stop;
        if (![selected containsIndex:idx]) {
            [kept addObject:url];
        }
    }];
    self.queue = kept;
    [self.queueTable reloadData];
}

- (void)typeChanged {
    NSArray<NSString *> *formats =
        self.audioRadio.state == NSControlStateValueOn ? AudioFormats() : VideoFormats();
    [self.formatPopup removeAllItems];
    [self.formatPopup addItemsWithTitles:formats];
    [self.formatPopup selectItemAtIndex:0];
}

- (void)browseOutput {
    NSString *initial = self.outputField.stringValue.length == 0 ? self.defaultDir : self.outputField.stringValue;
    [self browseDirectory:initial
                    title:@"Select output folder"
                   onPick:^(NSString *path) {
                       self.outputField.stringValue = path;
                   }];
}

- (void)browseSettingsDir {
    NSString *initial = self.settingsDirField.stringValue.length == 0
        ? [@"~/Downloads" stringByExpandingTildeInPath]
        : self.settingsDirField.stringValue;
    [self browseDirectory:initial
                    title:@"Select default download folder"
                   onPick:^(NSString *path) {
                       self.settingsDirField.stringValue = path;
                   }];
}

- (void)browseDirectory:(NSString *)initial title:(NSString *)title onPick:(void (^)(NSString *path))onPick {
    NSOpenPanel *panel = [NSOpenPanel openPanel];
    panel.canChooseDirectories = YES;
    panel.canChooseFiles = NO;
    panel.canCreateDirectories = YES;
    panel.allowsMultipleSelection = NO;
    panel.message = title;
    panel.directoryURL = [NSURL fileURLWithPath:QuarkNormalizedPath(initial) isDirectory:YES];
    [panel beginSheetModalForWindow:self.window
                  completionHandler:^(NSModalResponse response) {
                      if (response == NSModalResponseOK && panel.URL != nil) {
                          onPick(panel.URL.path);
                      }
                  }];
}

- (void)openSettings {
    [self showSettings];
}

- (void)closeSettings {
    [self showMain];
}

- (void)resetSettings {
    self.settingsDirField.stringValue = @"~/Downloads";
    QuarkSelect(self.themePopup, @"system", ThemeValues());
    self.stripCheck.state = NSControlStateValueOn;
    self.sanitizeCheck.state = NSControlStateValueOn;
    QuarkSelect(self.spacesPopup, @"keep", SpacesValues());
    self.playlistCheck.state = NSControlStateValueOn;
    QuarkSelect(self.modePopup, @"progress", ModeValues());
    self.logsCheck.state = NSControlStateValueOn;
    self.openOutputCheck.state = NSControlStateValueOff;
}

- (void)saveSettings {
    NSString *dir =
        [self.settingsDirField.stringValue stringByTrimmingCharactersInSet:NSCharacterSet.whitespaceAndNewlineCharacterSet];
    if (dir.length == 0) {
        QuarkShowModalError(@"Please choose a default download folder.", self.window);
        return;
    }

    NSString *previousDefault = self.defaultDir;
    NSString *normalizedDir = QuarkNormalizedPath(dir);
    NSString *currentOutput =
        [self.outputField.stringValue stringByTrimmingCharactersInSet:NSCharacterSet.whitespaceAndNewlineCharacterSet];

    self.downloadDir = dir;
    self.theme = QuarkNormalizeTheme(self.themePopup.titleOfSelectedItem ?: @"system");
    self.stripIds = self.stripCheck.state == NSControlStateValueOn;
    self.sanitize = self.sanitizeCheck.state == NSControlStateValueOn;
    self.spaces = self.spacesPopup.titleOfSelectedItem ?: @"keep";
    self.playlistFolders = self.playlistCheck.state == NSControlStateValueOn;
    self.guiMode = self.modePopup.titleOfSelectedItem ?: @"progress";
    self.logs = self.logsCheck.state == NSControlStateValueOn;
    self.openOutputDir = self.openOutputCheck.state == NSControlStateValueOn;
    self.defaultDir = normalizedDir;
    self.settingsSaved = YES;
    QuarkApplyTheme(self.theme);

    if (currentOutput.length == 0 || [currentOutput isEqualToString:previousDefault]) {
        self.outputField.stringValue = normalizedDir;
    }

    [self showMain];
}

- (void)checkUpdates {
    if (self.updateCheckRunning) {
        return;
    }

    NSString *gui = NSProcessInfo.processInfo.arguments.firstObject;
    if (![[NSFileManager defaultManager] isExecutableFileAtPath:gui]) {
        NSString *bundleExe = [NSBundle mainBundle].executablePath;
        if (bundleExe.length > 0) {
            gui = bundleExe;
        }
    }
    if (![[NSFileManager defaultManager] isExecutableFileAtPath:gui]) {
        QuarkShowModalError(@"quark-downloader-gui was not found.", self.window);
        return;
    }

    self.updateCheckRunning = YES;
    self.updatesButton.title = @"Checking…";
    self.updatesButton.enabled = NO;

    NSTask *process = [[NSTask alloc] init];
    process.executableURL = [NSURL fileURLWithPath:gui];
    process.arguments = @[@"--check-updates"];
    NSError *error = nil;
    if (![process launchAndReturnError:&error]) {
        [self resetUpdatesButton];
        NSString *body = [NSString stringWithFormat:@"Could not check for updates:\n%@", error.localizedDescription];
        QuarkShowModalError(body, self.window);
        return;
    }

    __weak typeof(self) weakSelf = self;
    dispatch_after(dispatch_time(DISPATCH_TIME_NOW, (int64_t)(1.5 * NSEC_PER_SEC)), dispatch_get_main_queue(), ^{
        [weakSelf resetUpdatesButton];
    });
}

- (void)resetUpdatesButton {
    self.updateCheckRunning = NO;
    self.updatesButton.title = @"Check for updates…";
    self.updatesButton.enabled = YES;
}

- (void)startDownload {
    [self addUrl];

    if (self.queue.count == 0) {
        QuarkShowModalError(@"Please enter at least one video or playlist URL.", self.window);
        return;
    }

    NSString *output =
        [self.outputField.stringValue stringByTrimmingCharactersInSet:NSCharacterSet.whitespaceAndNewlineCharacterSet];
    if (output.length == 0) {
        QuarkShowModalError(@"Please choose an output folder.", self.window);
        return;
    }

    NSString *mediaType = self.audioRadio.state == NSControlStateValueOn ? @"audio" : @"video";
    NSString *format = self.formatPopup.titleOfSelectedItem ?: @"original";
    [self emitDownloadWithUrls:self.queue mediaType:mediaType format:format output:output];
}

- (void)cancelSession {
    [self emitCancel];
}

- (BOOL)windowShouldClose:(NSWindow *)sender {
    (void)sender;
    [self emitCancel];
    return NO;
}

- (NSDictionary *)settingsObject {
    return @{
        @"download_dir": self.downloadDir,
        @"yt_dlp": @"path",
        @"ffmpeg": @"path",
        @"gui_download_mode": self.guiMode,
        @"download_logs": @(self.logs),
        @"gui_theme": self.theme,
        @"strip_video_ids": @(self.stripIds),
        @"sanitize_filenames": @(self.sanitize),
        @"filename_spaces": self.spaces,
        @"playlist_folders": @(self.playlistFolders),
        @"open_output_dir": @(self.openOutputDir),
    };
}

- (void)emitJSON:(NSDictionary *)object {
    NSError *err = nil;
    NSData *data = [NSJSONSerialization dataWithJSONObject:object options:0 error:&err];
    if (data == nil) {
        fputs("{}\n", stdout);
        exit(1);
    }
    fwrite(data.bytes, 1, data.length, stdout);
    fputc('\n', stdout);
    fflush(stdout);
    exit(0);
}

- (void)emitDownloadWithUrls:(NSArray<NSString *> *)urls
                   mediaType:(NSString *)mediaType
                      format:(NSString *)format
                      output:(NSString *)output {
    NSMutableDictionary *object = [@{
        @"v": @1,
        @"action": @"download",
        @"urls": urls,
        @"media_type": mediaType,
        @"format": format,
        @"output_dir": output,
    } mutableCopy];
    if (self.settingsSaved) {
        object[@"settings"] = [self settingsObject];
    }
    [self emitJSON:object];
}

- (void)emitCancel {
    NSMutableDictionary *object = [@{
        @"v": @1,
        @"action": @"cancel",
    } mutableCopy];
    if (self.settingsSaved) {
        object[@"settings"] = [self settingsObject];
    }
    [self emitJSON:object];
}

@end
