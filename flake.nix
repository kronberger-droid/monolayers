{
  description = "Rust CLI development shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    rust-overlay,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [rust-overlay.overlays.default];
        };

        rustTools = {
          stable = pkgs.rust-bin.stable.latest.default.override {
            extensions = ["rust-src"];
          };
          analyzer = pkgs.rust-bin.stable.latest.rust-analyzer;
        };

        devTools = with pkgs; [
          cargo-expand
          cargo-dist
          pkg-config
          gcc
        ];

        rustDeps =
          [
            rustTools.stable
            rustTools.analyzer
          ]
          ++ devTools;

        shellHook = ''
          echo "Using Rust toolchain: $(rustc --version)"
          export CARGO_HOME="$HOME/.cargo"
          export RUSTUP_HOME="$HOME/.rustup"
          mkdir -p "$CARGO_HOME" "$RUSTUP_HOME"
        '';
      in {
        devShells.default = pkgs.mkShell {
          name = "rust-cli-dev";
          buildInputs = rustDeps;
          inherit shellHook;
        };
      }
    );
}
