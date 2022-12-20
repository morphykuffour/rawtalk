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
    # PKG_CONFIG_PATH = concat [
    #   "${pkgs.libusb1.dev}/lib/pkgconfig"
    # ];

    # cmake flags pointing to locations of libusb1 headers and binaries
    # libusbDirs = libusb1: [
    #   "-DLIBUSB_1_INCLUDE_DIRS=${libusb1.dev}/include/libusb-1.0"
    #   "-DLIBUSB_1_LIBRARIES=${libusb1}/lib/libusb-1.0.so"
    # ];

    # Fix the USB backend library lookup
    postPatch = lib.optionalString stdenv.isLinux ''
      libusb=${pkgs.libusb1.dev}/include/libusb-1.0
      test -d $libusb || { echo "ERROR: $libusb doesn't exist, please update/fix this build expression."; exit 1; }
      sed -i -e "s|/usr/include/libusb-1.0|$libusb|" setup.py
    '';
  }
