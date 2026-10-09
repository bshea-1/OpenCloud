#import <Cocoa/Cocoa.h>

void setup_macos_app(const unsigned char *icon_png_data, size_t len) {
    @autoreleasepool {
        [[NSProcessInfo processInfo] setProcessName:@"OpenCloud"];
        if (icon_png_data && len > 0) {
            NSData *data = [NSData dataWithBytes:icon_png_data length:len];
            NSImage *image = [[NSImage alloc] initWithData:data];
            if (image) {
                [NSApplication sharedApplication].applicationIconImage = image;
            }
        }
        NSMenu *mainMenu = [NSApplication sharedApplication].mainMenu;
        if (mainMenu && [mainMenu numberOfItems] > 0) {
            NSMenuItem *appItem = [mainMenu itemAtIndex:0];
            [appItem setTitle:@"OpenCloud"];
            if ([appItem submenu]) {
                [[appItem submenu] setTitle:@"OpenCloud"];
            }
        }
    }
}
