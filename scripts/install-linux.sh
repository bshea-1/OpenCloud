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

BIN_DIR="$HOME/.local/bin"
DESKTOP_DIR="$HOME/.local/share/applications"
mkdir -p "$BIN_DIR" "$DESKTOP_DIR"

echo "==> OpenCloud Suite Installer for Linux"

for app in "${APPS[@]}"; do
  echo "--- Fetching latest release for $app ---"
  LATEST_JSON=$(curl -sSL "https://api.github.com/repos/storytold/${app}/releases/latest")
  
  ASSET_URL=$(echo "$LATEST_JSON" | jq -r '.assets[] | select(.name | test("linux-x86_64\\.AppImage$")) | .browser_download_url' | head -n 1)
  
  if [[ -z "$ASSET_URL" || "$ASSET_URL" == "null" ]]; then
    ASSET_URL=$(echo "$LATEST_JSON" | jq -r '.assets[] | select(.name | test("linux-x86_64\\.tar\\.gz$")) | .browser_download_url' | head -n 1)
  fi

  if [[ -z "$ASSET_URL" || "$ASSET_URL" == "null" ]]; then
    echo "  [!] No compatible Linux asset found for $app, skipping..."
    continue
  fi

  FILE_NAME=$(basename "$ASSET_URL")
  TARGET_PATH="$BIN_DIR/$app"

  echo "  --> Downloading $FILE_NAME..."
  if [[ "$FILE_NAME" == *".AppImage" ]]; then
    curl -sSL -o "$TARGET_PATH" "$ASSET_URL"
    chmod +x "$TARGET_PATH"
  else
    TEMP_ARCHIVE=$(mktemp)
    curl -sSL -o "$TEMP_ARCHIVE" "$ASSET_URL"
    tar -xzf "$TEMP_ARCHIVE" -C "$BIN_DIR"
    rm -f "$TEMP_ARCHIVE"
  fi

  # Create standard FreeDesktop .desktop entry
  cat <<EOF > "$DESKTOP_DIR/${app}.desktop"
[Desktop Entry]
Name=${app^}
Exec=$TARGET_PATH %U
Terminal=false
Type=Application
Categories=Graphics;Office;AudioVideo;Development;
EOF

  chmod +x "$DESKTOP_DIR/${app}.desktop"
  echo "  [✓] Installed $app to $TARGET_PATH and registered desktop launcher."
done

echo "==> Linux suite installation complete. Make sure $BIN_DIR is on your PATH."
