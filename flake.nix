{
  description = "A Nix-flake-based C/C++ development environment";

  inputs = {
    # nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.1.*.tar.gz";
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
    cargo2nix.url = "github:cargo2nix/cargo2nix";
  };

  outputs = inputs:
    with inputs;
      utils.lib.eachDefaultSystem (
        system: let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [cargo2nix.overlays.default];
          };

          rustPkgs = pkgs.rustBuilder.makePackageSet {
            rustVersion = "1.82.0";
            packageFun = import ./Cargo.nix;
          };
        in rec {
          devShells.default = pkgs.mkShell {
            packages = with pkgs; [
              cargo
              rustc
              nixd
              dbus.dev
              dbus.lib
              pkg-config
            ];
            shellHook = ''
              export PKG_CONFIG_PATH="${pkgs.dbus.dev}/lib/pkgconfig:$PKG_CONFIG_PATH"
              export LD_LIBRARY_PATH="${pkgs.dbus.lib}/lib:$LD_LIBRARY_PATH"
              export CMAKE_PREFIX_PATH="${pkgs.dbus.dev}:$CMAKE_PREFIX_PATH"
            '';
          };

          packages = {
            bluez-test = rustPkgs.workspace.bluez-test {};

            default = packages.bluez-test;
          };
        }
      );
}
