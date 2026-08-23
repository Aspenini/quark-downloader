#import "ProgressWindow.h"
#import "QuarkMac.h"

#import <stdio.h>
#import <stdlib.h>
#import <string.h>

static const NSTimeInterval kEtaUpdateInterval = 1.5;

@interface ProgressController ()
@property (nonatomic, strong) NSWindow *window;
@property (nonatomic, strong) NSTextField *statusLabel;
@property (nonatomic, strong) NSTextField *queueLabel;
@property (nonatomic, strong) NSProgressIndicator *bar;
@property (nonatomic, strong) NSTextField *etaLabel;
@property (nonatomic, copy) NSString *eta;
@property (nonatomic) NSTimeInterval lastEtaUpdate;
@property (nonatomic) NSUInteger etaGeneration;
@property (nonatomic) BOOL hasPendingEta;
@property (nonatomic) BOOL finished;
@property (nonatomic, strong) id keyMonitor;
@end

@implementation ProgressController

- (void)show {
    self.eta = @"";
    self.window = [[NSWindow alloc]
        initWithContentRect:NSMakeRect(0, 0, 420, 130)
                  styleMask:(NSWindowStyleMaskTitled | NSWindowStyleMaskMiniaturizable)
                    backing:NSBackingStoreBuffered
                      defer:NO];
    self.window.delegate = self;
    self.window.releasedWhenClosed = NO;
    [self updateTitle];

    self.statusLabel = [NSTextField labelWithString:@"Starting download..."];
    self.statusLabel.lineBreakMode = NSLineBreakByTruncatingTail;
    self.queueLabel = [NSTextField labelWithString:@""];
    self.queueLabel.textColor = NSColor.secondaryLabelColor;
    self.queueLabel.lineBreakMode = NSLineBreakByTruncatingTail;
    self.etaLabel = [NSTextField labelWithString:@"Time left: estimating..."];
    self.etaLabel.lineBreakMode = NSLineBreakByTruncatingTail;

    self.bar = [[NSProgressIndicator alloc] init];
    self.bar.style = NSProgressIndicatorStyleBar;
    self.bar.indeterminate = NO;
    self.bar.minValue = 0;
    self.bar.maxValue = 100;
    self.bar.doubleValue = 0;

    NSStackView *stack = [NSStackView stackViewWithViews:@[
        self.statusLabel, self.queueLabel, self.bar, self.etaLabel
    ]];
    stack.orientation = NSUserInterfaceLayoutOrientationVertical;
    stack.alignment = NSLayoutAttributeLeading;
    stack.spacing = 8;
    stack.translatesAutoresizingMaskIntoConstraints = NO;

    NSView *content = [[NSView alloc] init];
    [content addSubview:stack];
    [NSLayoutConstraint activateConstraints:@[
        [stack.topAnchor constraintEqualToAnchor:content.topAnchor constant:14],
        [stack.bottomAnchor constraintEqualToAnchor:content.bottomAnchor constant:-14],
        [stack.leadingAnchor constraintEqualToAnchor:content.leadingAnchor constant:14],
        [stack.trailingAnchor constraintEqualToAnchor:content.trailingAnchor constant:-14],
        [content.widthAnchor constraintEqualToConstant:420],
        [self.bar.widthAnchor constraintEqualToAnchor:stack.widthAnchor],
        [self.statusLabel.widthAnchor constraintEqualToAnchor:stack.widthAnchor],
        [self.queueLabel.widthAnchor constraintEqualToAnchor:stack.widthAnchor],
    ]];

    self.window.contentView = content;
    [content layoutSubtreeIfNeeded];
    [self.window setContentSize:content.fittingSize];
    [self.window center];
    [self.window makeKeyAndOrderFront:nil];

    self.keyMonitor = [NSEvent addLocalMonitorForEventsMatchingMask:NSEventMaskKeyDown
                                                            handler:^NSEvent *(NSEvent *event) {
        if (event.keyCode == 53) {
            exit(1);
        }
        return event;
    }];

    [self startReadingStdin];
}

- (void)startReadingStdin {
    __weak typeof(self) weakSelf = self;
    dispatch_async(dispatch_get_global_queue(QOS_CLASS_UTILITY, 0), ^{
        char *line = NULL;
        size_t cap = 0;
        ssize_t nread;
        while ((nread = getline(&line, &cap, stdin)) >= 0) {
            size_t len = strcspn(line, "\r\n");
            NSString *text = [[NSString alloc] initWithBytes:line length:len encoding:NSUTF8StringEncoding] ?: @"";
            dispatch_async(dispatch_get_main_queue(), ^{
                [weakSelf applyLine:text];
            });
        }
        free(line);
        dispatch_async(dispatch_get_main_queue(), ^{
            if (!weakSelf.finished) {
                exit(1);
            }
        });
    });
}

- (void)applyLine:(NSString *)line {
    if (self.finished) {
        return;
    }
    NSRange tab = [line rangeOfString:@"\t"];
    NSString *kind = line;
    NSString *payload = @"";
    if (tab.location != NSNotFound) {
        kind = [line substringToIndex:tab.location];
        payload = [line substringFromIndex:tab.location + 1];
    }

    if ([kind isEqualToString:@"PROGRESS"]) {
        self.bar.doubleValue = MIN(MAX(payload.doubleValue, 0.0), 100.0);
    } else if ([kind isEqualToString:@"ETA"]) {
        self.eta = payload;
        [self scheduleEtaDisplayUpdate];
    } else if ([kind isEqualToString:@"STATUS"]) {
        self.statusLabel.stringValue = payload;
    } else if ([kind isEqualToString:@"QUEUE"]) {
        self.queueLabel.stringValue = payload;
    } else if ([kind isEqualToString:@"DONE"]) {
        self.finished = YES;
        self.etaGeneration += 1;
        self.hasPendingEta = NO;
        int code = payload.length ? payload.intValue : 1;
        exit(code);
    }
}

- (void)scheduleEtaDisplayUpdate {
    NSTimeInterval now = [NSDate date].timeIntervalSince1970;
    if (self.lastEtaUpdate == 0 || now - self.lastEtaUpdate >= kEtaUpdateInterval) {
        self.hasPendingEta = NO;
        self.etaGeneration += 1;
        [self applyEtaDisplayUpdate];
        return;
    }
    if (self.hasPendingEta) {
        return;
    }
    self.hasPendingEta = YES;
    NSUInteger gen = ++self.etaGeneration;
    NSTimeInterval delay = kEtaUpdateInterval - (now - self.lastEtaUpdate);
    __weak typeof(self) weakSelf = self;
    dispatch_after(dispatch_time(DISPATCH_TIME_NOW, (int64_t)(delay * NSEC_PER_SEC)), dispatch_get_main_queue(), ^{
        ProgressController *self = weakSelf;
        if (self == nil || gen != self.etaGeneration) {
            return;
        }
        self.hasPendingEta = NO;
        [self applyEtaDisplayUpdate];
    });
}

- (void)applyEtaDisplayUpdate {
    if (self.eta.length == 0) {
        self.etaLabel.stringValue = @"Time left: estimating...";
    } else {
        self.etaLabel.stringValue = [NSString stringWithFormat:@"Time left: %@ left", self.eta];
    }
    [self updateTitle];
    self.lastEtaUpdate = [NSDate date].timeIntervalSince1970;
}

- (void)updateTitle {
    if (self.eta.length == 0) {
        self.window.title = [NSString stringWithFormat:@"%@ - estimating...", QuarkAppWindowTitle()];
    } else {
        self.window.title = [NSString stringWithFormat:@"%@ - %@ left", QuarkAppWindowTitle(), self.eta];
    }
}

@end
