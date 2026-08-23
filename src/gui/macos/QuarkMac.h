#pragma once

#import <AppKit/AppKit.h>

NSString *QuarkAppWindowTitle(void);
NSString *QuarkAppSettingsWindowTitle(void);
NSString *QuarkNormalizeTheme(NSString *value);
void QuarkApplyTheme(NSString *theme);
void QuarkInstallMainMenu(void);
void QuarkShowModalError(NSString *message, NSWindow *window);
int QuarkRunMessageAlert(NSString *kind, NSString *title, NSString *body);
NSString *QuarkNormalizedPath(NSString *path);
BOOL QuarkParseBool(NSString *value, BOOL fallback);
NSString *QuarkArg(const char *s);
