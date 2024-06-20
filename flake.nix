{
  description = "a653rs-router";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
    utils.url = "github:numtide/flake-utils";
    devshell = {
      url = "github:numtide/devshell";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    hypervisor = {
      url = "github:DLR-FT/a653rs-linux";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.devshell.follows = "devshell";
      inputs.fenix.follows = "fenix";
      inputs.utils.follows = "utils";
    };
    xng-utils = {
      url = "github:aeronautical-informatics/xng-flake-utils";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, utils, devshell, fenix, hypervisor, xng-utils, ... }@inputs:
    utils.lib.eachSystem [ "x86_64-linux" ]
      (system:
        let
          pkgs = import nixpkgs { inherit system; };
          cargoLock = import ./cargo-lock.nix;
          formatter = pkgs.nixpkgs-fmt;
          a653rs-linux-hypervisor = hypervisor.packages.${system}.a653rs-linux-hypervisor;
          lithOsOps = import ./lithos-ops.nix { inherit pkgs xng-utils; };
          partitions = self.packages.${system};
          runTest = nixpkgs.lib.nixos.runTest;
          rustToolchain = import ./rust-toolchain.nix {
            fenix = fenix.packages.${system};
          };
          rustPlatform = (pkgs.makeRustPlatform { cargo = rustToolchain; rustc = rustToolchain; });
          xngOps = import ./xng-ops.nix { inherit pkgs xng-utils; };
        in
        {
          inherit formatter;

          devShells =
            let
              pkgs = import nixpkgs { inherit system; overlays = [ devshell.overlays.default ]; };
            in
            {
              default = import ./shell.nix {
                inherit pkgs devshell formatter rustToolchain;
              };
            };

          checks = import ./checks.nix {
            inherit pkgs a653rs-linux-hypervisor partitions runTest rustToolchain;
          };

          packages =
            {
              a653rs-router-cfg = import ./a653rs-router-cfg { inherit cargoLock rustPlatform; };
            } // (import ./partitions.nix {
              inherit pkgs cargoLock rustPlatform;
            }) // (import ./xng-images.nix {
              inherit pkgs lithOsOps partitions xngOps xng-utils;

              a653rs-router-cfg = self.packages.${system}.a653rs-router-cfg;
            });
        }
      ); # outputs
}
