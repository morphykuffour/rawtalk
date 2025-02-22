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

        # Create udev rules
        udevRules = pkgs.writeTextFile {
          name = "70-qmk-ferris.rules";
          text = ''
            SUBSYSTEMS=="usb", ATTRS{idVendor}=="c2ab", ATTRS{idProduct}=="3939", TAG+="uaccess"
            KERNEL=="hidraw*", ATTRS{idVendor}=="c2ab", ATTRS{idProduct}=="3939", TAG+="uaccess"
          '';
          destination = "/etc/udev/rules.d/70-qmk-ferris.rules";
        };

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

          # Install udev rules during package installation
          postInstall = ''
            mkdir -p $out/lib/udev/rules.d
            cp ${udevRules}/etc/udev/rules.d/* $out/lib/udev/rules.d/
          '';
        };
      }
    );
}
