// Quark Downloader — native macOS UI (compiled into quark-downloader-gui).
// Speaks the same argv/stdout/stdin protocol as the Qt frontend.

#import "QuarkMac.h"
#import "ProgressWindow.h"
#import "SessionWindow.h"

#import <stdio.h>

NSString *QuarkArg(const char *s) {
    if (s == NULL) {
        return @"";
    }
    NSString *text = [NSString stringWithUTF8String:s];
    return text ?: @"";
}

NSString *QuarkAppWindowTitle(void) {
    NSString *version = NSProcessInfo.processInfo.environment[@"QUARK_VERSION"] ?: @"";
    if (version.length == 0) {
        return @"Quark Downloader";
    }
    return [NSString stringWithFormat:@"Quark Downloader %@", version];
}

NSString *QuarkAppSettingsWindowTitle(void) {
    return [NSString stringWithFormat:@"%@ Settings", QuarkAppWindowTitle()];
}

NSString *QuarkNormalizeTheme(NSString *value) {
    NSString *lower = value.lowercaseString;
    if ([lower isEqualToString:@"dark"]) {
        return @"dark";
    }
    if ([lower isEqualToString:@"light"]) {
        return @"light";
    }
    return @"system";
}

void QuarkApplyTheme(NSString *theme) {
    NSString *normalized = QuarkNormalizeTheme(theme);
    if ([normalized isEqualToString:@"dark"]) {
        NSApp.appearance = [NSAppearance appearanceNamed:NSAppearanceNameDarkAqua];
    } else if ([normalized isEqualToString:@"light"]) {
        NSApp.appearance = [NSAppearance appearanceNamed:NSAppearanceNameAqua];
    } else {
        NSApp.appearance = nil;
    }
}

NSString *QuarkNormalizedPath(NSString *path) {
    NSString *expanded = [path stringByExpandingTildeInPath];
    return [[[NSURL fileURLWithPath:expanded] URLByStandardizingPath] path];
}

BOOL QuarkParseBool(NSString *value, BOOL fallback) {
    if (value.length == 0) {
        return fallback;
    }
    NSString *lower = value.lowercaseString;
    return [@[@"true", @"1", @"yes", @"on"] containsObject:lower];
}

void QuarkInstallMainMenu(void) {
    // Without a main menu, standard edit shortcuts (Cmd+V etc.) do not work.
    NSMenu *mainMenu = [[NSMenu alloc] init];

    NSMenuItem *appItem = [[NSMenuItem alloc] init];
    [mainMenu addItem:appItem];
    NSMenu *appMenu = [[NSMenu alloc] init];
    [appMenu addItemWithTitle:@"Quit Quark Downloader"
                       action:@selector(terminate:)
                keyEquivalent:@"q"];
    appItem.submenu = appMenu;

    NSMenuItem *editItem = [[NSMenuItem alloc] init];
    [mainMenu addItem:editItem];
    NSMenu *editMenu = [[NSMenu alloc] initWithTitle:@"Edit"];
    [editMenu addItemWithTitle:@"Undo" action:NSSelectorFromString(@"undo:") keyEquivalent:@"z"];
    [editMenu addItemWithTitle:@"Redo" action:NSSelectorFromString(@"redo:") keyEquivalent:@"Z"];
    [editMenu addItem:[NSMenuItem separatorItem]];
    [editMenu addItemWithTitle:@"Cut" action:@selector(cut:) keyEquivalent:@"x"];
    [editMenu addItemWithTitle:@"Copy" action:@selector(copy:) keyEquivalent:@"c"];
    [editMenu addItemWithTitle:@"Paste" action:@selector(paste:) keyEquivalent:@"v"];
    [editMenu addItemWithTitle:@"Select All" action:@selector(selectAll:) keyEquivalent:@"a"];
    editItem.submenu = editMenu;

    NSApp.mainMenu = mainMenu;
}

static id gController = nil;

int appkit_ui_run(int argc, char **argv) {
    @autoreleasepool {
        [NSApplication sharedApplication];
        [NSApp setActivationPolicy:NSApplicationActivationPolicyRegular];
        QuarkInstallMainMenu();

        if (argc < 2) {
            fputs("usage: --session|--progress|--message\n", stderr);
            return 2;
        }

        NSString *mode = QuarkArg(argv[1]);
        if ([mode isEqualToString:@"--message"]) {
            if (argc < 5) {
                fputs("usage: --message <ok|error> <title> <body>\n", stderr);
                return 2;
            }
            NSMutableString *body = [NSMutableString string];
            for (int i = 4; i < argc; i++) {
                if (i > 4) {
                    [body appendString:@" "];
                }
                [body appendString:QuarkArg(argv[i])];
            }
            return QuarkRunMessageAlert(QuarkArg(argv[2]), QuarkArg(argv[3]), body);
        }

        if ([mode isEqualToString:@"--session"]) {
            NSMutableArray<NSString *> *args = [NSMutableArray array];
            for (int i = 2; i < argc; i++) {
                [args addObject:QuarkArg(argv[i])];
            }
            SessionController *session = [[SessionController alloc] initWithArguments:args];
            QuarkApplyTheme(session.theme);
            gController = session;
            [session show];
            [NSApp activateIgnoringOtherApps:YES];
            [NSApp run];
            return 0;
        }

        if ([mode isEqualToString:@"--progress"]) {
            NSString *theme = @"system";
            for (int i = 2; i < argc; i++) {
                NSString *lower = QuarkArg(argv[i]).lowercaseString;
                if ([lower isEqualToString:@"light"] || [lower isEqualToString:@"dark"]
                    || [lower isEqualToString:@"system"]) {
                    theme = lower;
                }
            }
            QuarkApplyTheme(theme);
            ProgressController *progress = [[ProgressController alloc] init];
            gController = progress;
            [progress show];
            [NSApp activateIgnoringOtherApps:YES];
            [NSApp run];
            return 0;
        }

        fputs("usage: --session ... | --progress ... | --message ...\n", stderr);
        return 2;
    }
}
