{
  description = "A Nix-flake-based C/C++ development environment";

  inputs = {
    # nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.1.*.tar.gz";
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    ...
  } @ inputs:
    inputs.utils.lib.eachSystem [
      "x86_64-linux"
      "i686-linux"
      "aarch64-linux"
      "x86_64-darwin"
    ] (system: let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [];
        # config.allowUnfree = true;
      };
    in {
      devShells.default = pkgs.mkShell {
        packages = with pkgs; [
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
    });
}
