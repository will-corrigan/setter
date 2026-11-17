{
  description = "Setter: the webview libraries for building on NixOS (Rust and dx come from mise.toml)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        libs = with pkgs; [
          webkitgtk_4_1
          gtk3
          glib
          libsoup_3
          librsvg
          openssl
          cairo
          pango
          gdk-pixbuf
          atk
          harfbuzz
          glib-networking
          wayland
          libxkbcommon
          libGL
          fontconfig
          freetype
          xdotool
        ];
      in
      {
        devShells.default = pkgs.mkShell {
          # Rust and dx come from mise (mise.toml) on every OS; this shell only
          # adds what NixOS can't find system-wide, plus a Python for scripts/.
          nativeBuildInputs = with pkgs; [
            pkg-config
            clang
            mold
            python3
          ];
          buildInputs = libs;

          shellHook = ''
            # Correct display scaling / schemas on Wayland (incl. WSLg)
            export XDG_DATA_DIRS="$GSETTINGS_SCHEMAS_PATH:$XDG_DATA_DIRS"
            export GIO_MODULE_DIR="${pkgs.glib-networking}/lib/gio/modules/"
            # WebKitGTK's DMA-BUF renderer is flaky under WSLg / some GPUs
            export WEBKIT_DISABLE_DMABUF_RENDERER=1
            # Nix sets this for reproducible builds; the AppImage packer
            # (mksquashfs) refuses it alongside its own timestamp options.
            unset SOURCE_DATE_EPOCH
            export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath libs}:/usr/lib/wsl/lib:$LD_LIBRARY_PATH"
            # gcc's collect2 hits "Argument list too long" with this shell's huge NIX_CFLAGS_COMPILE
            export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=clang
            export RUSTFLAGS="-C link-arg=-fuse-ld=mold"
          '';
        };
      });
}
