#!/bin/sh
# Run a command where Setter's desktop build can find the webview libraries.
# On NixOS they come from flake.nix (a nix develop shell); everywhere else
# they're installed system-wide (mise bootstrap packages apply), or built into
# the OS, so the command just runs.
root="$(cd "$(dirname "$0")/.." && pwd)"
if [ -e /etc/NIXOS ] && [ -z "$IN_NIX_SHELL" ]; then
  exec nix develop "$root" --command "$@"
fi
exec "$@"
