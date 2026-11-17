#!/bin/sh
# Linux only: install a launcher (.desktop file) and icon for running Setter
# from this checkout. Desktops, and WSLg's Windows taskbar and Start menu,
# take a window's icon from the launcher whose name matches the app's ID
# ("setter"); without one, WSLg shows a generic penguin.
# Remove with: scripts/install-desktop-entry.sh --remove
set -eu
root="$(cd "$(dirname "$0")/.." && pwd)"
apps="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
icons="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor"
if [ "${1:-}" = "--remove" ]; then
  command rm -f "$apps/setter.desktop" "$icons/256x256/apps/setter.png" "$icons/128x128/apps/setter.png" "$icons/32x32/apps/setter.png"
  echo "Removed the Setter launcher."
  exit 0
fi
mkdir -p "$apps" "$icons/256x256/apps" "$icons/128x128/apps" "$icons/32x32/apps"
command cp -f "$root/app/icons/128x128@2x.png" "$icons/256x256/apps/setter.png"
command cp -f "$root/app/icons/128x128.png" "$icons/128x128/apps/setter.png"
command cp -f "$root/app/icons/32x32.png" "$icons/32x32/apps/setter.png"
cat > "$apps/setter.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=Setter
Comment=Set British cryptic crosswords
Exec=$root/scripts/in-env.sh $root/target/dx/setter/debug/linux/app/setter
Icon=$icons/256x256/apps/setter.png
StartupWMClass=setter
Categories=Game;
Terminal=false
DESKTOP
echo "Installed $apps/setter.desktop"
