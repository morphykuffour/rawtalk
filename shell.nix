{ pkgs ? import <nixpkgs> { } }:
let
  dependencies = with pkgs; [
    pkg-config
    libusb1
    hidapi
  ];
in
with pkgs;
mkShell {

  dependencies = dependencies;

  packages = [
    dependencies
    # cowsay 
  ];

  buildInputs = [
    dependencies
  ];

  nativeBuildInputs = [
    pkgs.libusb1
  ];
}
