{
  description = "Cryptic crossword setter — Rust + Tauri desktop app";

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
        ];
      in
      {
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            pkg-config
            rustc
            cargo
            clippy
            rustfmt
            rust-analyzer
            cargo-tauri
            nodejs_22
            python3
          ];
          buildInputs = libs;

          shellHook = ''
            # Correct display scaling / schemas on Wayland (incl. WSLg)
            export XDG_DATA_DIRS="$GSETTINGS_SCHEMAS_PATH:$XDG_DATA_DIRS"
            export GIO_MODULE_DIR="${pkgs.glib-networking}/lib/gio/modules/"
            # WebKitGTK's DMA-BUF renderer is flaky under WSLg / some GPUs
            export WEBKIT_DISABLE_DMABUF_RENDERER=1
            export RUST_SRC_PATH="${pkgs.rustPlatform.rustLibSrc}"
          '';
        };
      });
}
