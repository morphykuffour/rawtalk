{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, rust-overlay, ... }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f system);
    in {
      packages = forAllSystems (system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ rust-overlay.overlays.default ];
          };
          rustPlatform = pkgs.makeRustPlatform {
            cargo = pkgs.rust-bin.stable.latest.default;
            rustc = pkgs.rust-bin.stable.latest.default;
          };
        in {
          default = rustPlatform.buildRustPackage {
            pname = "rawtalk";
            version = "0.4.0";
            src = self;
            cargoLock.lockFile = ./Cargo.lock;
            nativeBuildInputs = [ pkgs.pkg-config ];
            buildInputs = [ pkgs.hidapi ]
              ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux [ pkgs.systemd ];
            # On Darwin, hidapi already links to the necessary frameworks
            meta = {
              description = "QMK Layer Switcher";
              homepage = "https://github.com/morphykuffour/rawtalk";
              mainProgram = "rawtalk";
            };
          };
          rawtalk = self.packages.${system}.default;
        });

      devShells = forAllSystems (system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ rust-overlay.overlays.default ];
          };
        in {
          default = pkgs.mkShell {
            nativeBuildInputs = [ pkgs.pkg-config pkgs.rust-bin.stable.latest.default ];
            buildInputs = [ pkgs.hidapi ]
              ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux [ pkgs.systemd ];
          };
        });
    };
}
