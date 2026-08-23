#pragma once

#import <AppKit/AppKit.h>

@interface SessionController : NSObject <NSWindowDelegate, NSTableViewDataSource, NSTableViewDelegate>
@property (nonatomic, copy, readonly) NSString *theme;
- (instancetype)initWithArguments:(NSArray<NSString *> *)arguments;
- (void)show;
@end
