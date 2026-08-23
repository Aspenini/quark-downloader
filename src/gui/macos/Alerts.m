#import "QuarkMac.h"

int QuarkRunMessageAlert(NSString *kind, NSString *title, NSString *body) {
    NSAlert *alert = [[NSAlert alloc] init];
    alert.messageText = title;
    alert.informativeText = body;
    alert.alertStyle = [kind isEqualToString:@"error"] ? NSAlertStyleCritical : NSAlertStyleInformational;
    [alert addButtonWithTitle:@"OK"];
    [NSApp activateIgnoringOtherApps:YES];
    [alert runModal];
    return 0;
}

void QuarkShowModalError(NSString *message, NSWindow *window) {
    NSAlert *alert = [[NSAlert alloc] init];
    alert.messageText = @"Quark Downloader";
    alert.informativeText = message;
    alert.alertStyle = NSAlertStyleCritical;
    [alert addButtonWithTitle:@"OK"];
    if (window != nil) {
        [alert beginSheetModalForWindow:window completionHandler:nil];
    } else {
        [alert runModal];
    }
}
