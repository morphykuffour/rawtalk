{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { nixpkgs, rust-overlay, ... }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f system);
    in {
      devShells = forAllSystems (system:
        let
          pkgs = import nixpkgs { inherit system; overlays = [ rust-overlay.default ]; };
        in {
          default = pkgs.mkShell {
            nativeBuildInputs = [ pkgs.pkg-config pkgs.rust-bin.stable.latest.default ];
            buildInputs = [ pkgs.hidapi ] ++ pkgs.lib.optionals pkgs.stdenv.isLinux [ pkgs.systemd ];
          };
        });
    };
}
