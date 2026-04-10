{
  description = "Monolayers — file immutability daemon";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {self, nixpkgs, fenix, ...}: let
    forAllSystems = nixpkgs.lib.genAttrs ["x86_64-linux" "aarch64-linux"];
  in {
    packages = forAllSystems (system: let
      pkgs = nixpkgs.legacyPackages.${system};
      common = {
        version = "0.1.0";
        src = ./.;
        cargoLock.lockFile = ./Cargo.lock;
        nativeBuildInputs = [pkgs.pkg-config];
      };
    in {
      monolayers-server = pkgs.rustPlatform.buildRustPackage (common // {
        pname = "monolayers-server";
        cargoBuildFlags = ["-p" "monolayers-server"];
      });

      monolayers-client = pkgs.rustPlatform.buildRustPackage (common // {
        pname = "monolayers-client";
        cargoBuildFlags = ["-p" "monolayers-client"];
      });

      default = self.packages.${system}.monolayers-server;
    });

    devShells = forAllSystems (system: let
      pkgs = nixpkgs.legacyPackages.${system};
      toolchain = fenix.packages.${system}.stable.withComponents [
        "cargo" "clippy" "rust-src" "rustc" "rustfmt"
      ];
    in {
      default = pkgs.mkShell {
        nativeBuildInputs = [
          toolchain
          fenix.packages.${system}.rust-analyzer
          pkgs.pkg-config
          pkgs.cargo-expand
        ];
      };
    });
  };
}
