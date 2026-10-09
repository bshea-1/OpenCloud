#!/usr/bin/env bash
set -euo pipefail

APPS=(
  "photocraft"
  "vectorcraft"
  "filmcraft"
  "lightcraft"
  "pdfcraft"
  "effectcraft"
  "designcraft"
  "soundcraft"
  "cadcraft"
  "deckcraft"
  "gridcraft"
  "wordcraft"
)

TEMP_DIR=$(mktemp -d)
trap 'rm -rf "$TEMP_DIR"' EXIT

echo "==> OpenCloud Suite Installer for macOS"
for app in "${APPS[@]}"; do
  echo "--- Checking latest release for $app ---"
  LATEST_JSON=$(curl -sSL "https://api.github.com/repos/storytold/${app}/releases/latest")
  DMG_URL=$(echo "$LATEST_JSON" | jq -r '.assets[] | select(.name | test("macos-universal\\.dmg$")) | .browser_download_url' | head -n 1)

  if [[ -z "$DMG_URL" || "$DMG_URL" == "null" ]]; then
    echo "  [!] Universal DMG not found for $app, skipping..."
    continue
  fi

  DMG_NAME=$(basename "$DMG_URL")
  echo "  --> Downloading $DMG_NAME..."
  curl -sSL -o "$TEMP_DIR/$DMG_NAME" "$DMG_URL"

  echo "  --> Mounting and installing..."
  MOUNT_DIR=$(mktemp -d)
  hdiutil attach "$TEMP_DIR/$DMG_NAME" -mountpoint "$MOUNT_DIR" -nobrowse -quiet
  
  # Copy .app bundle to /Applications
  APP_BUNDLE=$(find "$MOUNT_DIR" -maxdepth 2 -name "*.app" | head -n 1)
  if [[ -n "$APP_BUNDLE" ]]; then
    TARGET_APP="/Applications/$(basename "$APP_BUNDLE")"
    rm -rf "$TARGET_APP"
    cp -R "$APP_BUNDLE" "/Applications/"
    xattr -dr com.apple.quarantine "$TARGET_APP" 2>/dev/null || true
    echo "  [✓] Installed $(basename "$APP_BUNDLE") to /Applications/"
  fi

  hdiutil detach "$MOUNT_DIR" -quiet || true
  rm -rf "$MOUNT_DIR"
done

echo "==> Installation complete! All suite applications are ready in /Applications."
