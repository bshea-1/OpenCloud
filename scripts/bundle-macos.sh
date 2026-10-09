#!/usr/bin/env bash
set -euo pipefail

# Build the OpenCloud release or debug binary
MODE="${1:-release}"
if [ "$MODE" = "release" ]; then
    echo "==> Building OpenCloud in release mode..."
    cargo build --release --bin OpenCloud
    BIN_PATH="target/release/OpenCloud"
else
    echo "==> Building OpenCloud in debug mode..."
    cargo build --bin OpenCloud
    BIN_PATH="target/debug/OpenCloud"
fi

APP_DIR="target/OpenCloud.app"
CONTENTS_DIR="$APP_DIR/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
RESOURCES_DIR="$CONTENTS_DIR/Resources"

echo "==> Packaging $APP_DIR..."
rm -rf "$APP_DIR"
mkdir -p "$MACOS_DIR" "$RESOURCES_DIR"

# Copy binary
cp "$BIN_PATH" "$MACOS_DIR/OpenCloud"
chmod +x "$MACOS_DIR/OpenCloud"

# Copy icon
if [ -f "assets/icons/AppIcon.icns" ]; then
    cp "assets/icons/AppIcon.icns" "$RESOURCES_DIR/AppIcon.icns"
fi

# Create Info.plist
cat << 'EOF' > "$CONTENTS_DIR/Info.plist"
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>OpenCloud</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>CFBundleIdentifier</key>
    <string>com.opencloud.launcher</string>
    <key>CFBundleName</key>
    <string>OpenCloud</string>
    <key>CFBundleDisplayName</key>
    <string>OpenCloud</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0.0</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>LSMinimumSystemVersion</key>
    <string>12.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
EOF

echo "==> Successfully created $APP_DIR"

# Clean all extended attributes and properly ad-hoc codesign the bundle
echo "==> Signing $APP_DIR with ad-hoc signature..."
xattr -cr "$APP_DIR"
codesign --force --deep --sign - "$APP_DIR"
codesign -vvv --deep --strict "$APP_DIR"

if [ "$MODE" = "release" ]; then
    DMG_NAME="OpenCloud-1.0.0-macos-universal.dmg"
    DMG_PATH="target/$DMG_NAME"
    STAGING_DIR="target/dmg_staging"
    echo "==> Creating draggable macOS DMG installer ($DMG_NAME)..."
    rm -rf "$STAGING_DIR" "$DMG_PATH"
    mkdir -p "$STAGING_DIR"
    cp -R "$APP_DIR" "$STAGING_DIR/"
    ln -s /Applications "$STAGING_DIR/Applications"
    chmod -R a+rX "$STAGING_DIR/OpenCloud.app"
    xattr -cr "$STAGING_DIR"
    codesign --force --deep --sign - "$STAGING_DIR/OpenCloud.app"
    hdiutil create -volname "OpenCloud" -srcfolder "$STAGING_DIR" -ov -format UDZO "$DMG_PATH"
    xattr -cr "$DMG_PATH"
    codesign --force --sign - "$DMG_PATH" 2>/dev/null || true
    rm -rf "$STAGING_DIR"
    echo "==> Successfully generated signed draggable DMG: $DMG_PATH"
fi
