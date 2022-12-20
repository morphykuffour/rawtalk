{pkgs ? import <nixpkgs> {}}:
with pkgs;
  mkShell {
    nativeBuildInputs = [
      pkg-config
      libusb1
    ];

    buildInputs = [
      hidapi
      libusb1
      rustc
      clippy
      cargo
      rustfmt
      rust-analyzer
    ];

    RUST_BACKTRACE = 1;
  }
