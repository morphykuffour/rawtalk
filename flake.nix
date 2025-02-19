{
  description = "use hidapi to communicate with qmk keyboard";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        
        nativeBuildInputs = with pkgs; [
          pkg-config
          rust-bin.stable.latest.default
        ];
        
        buildInputs = with pkgs; [
          hidapi
        ];

      in {
        devShells.default = pkgs.mkShell {
          inherit nativeBuildInputs buildInputs;
        };

        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "qmk-layer-switcher";
          version = "0.1.0";
          src = ./.;
          
          inherit nativeBuildInputs buildInputs;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          installPhase = ''
            mkdir -p $out/bin
            cp target/${pkgs.stdenv.hostPlatform.config}/release/qmk-layer-switcher $out/bin/
          '';
        };
      }
    );
}
