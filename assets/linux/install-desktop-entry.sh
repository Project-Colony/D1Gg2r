#!/bin/sh
# Install Digger desktop entry and icon for Linux
# This enables the app icon to appear on Wayland compositors and desktop environments

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# Install icon
mkdir -p "$HOME/.local/share/icons/hicolor/256x256/apps"
cp "$SCRIPT_DIR/digger.png" "$HOME/.local/share/icons/hicolor/256x256/apps/digger.png"

# Install desktop entry
mkdir -p "$HOME/.local/share/applications"
cp "$SCRIPT_DIR/digger.desktop" "$HOME/.local/share/applications/digger.desktop"

# Update icon cache if available
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true
fi

echo "Desktop entry and icon installed successfully."
echo "You may need to log out and back in for the icon to appear."
