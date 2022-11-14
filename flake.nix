{
  description = "network-partition";

  inputs = {
    utils.url = "github:numtide/flake-utils";

    devshell.url = "github:numtide/devshell";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    hypervisor.url = "github:aeronautical-informatics/apex-linux/main";
  };

  outputs = { self, nixpkgs, utils, devshell, fenix, hypervisor, naersk, ... }@inputs:
    utils.lib.eachSystem [ "x86_64-linux" ] (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ devshell.overlay ];
        };
        formatter = pkgs.nixpkgs-fmt;
        rust-toolchain = with fenix.packages.${system}; combine [
          latest.rustc
          latest.cargo
          latest.clippy
          latest.rustfmt
          targets.x86_64-unknown-linux-musl.latest.rust-std
          targets.thumbv6m-none-eabi.latest.rust-std
        ];
        naerskLib = (naersk.lib.${system}.override {
          cargo = rust-toolchain;
          rustc = rust-toolchain;
        });
        hypervisorPackage = hypervisor.packages.${system}.linux-apex-hypervisor;
      in
      {
        inherit formatter;

        devShells.default = pkgs.devshell.mkShell {
          imports = [ "${devshell}/extra/git/hooks.nix" ];
          name = "network-partition";
          packages = with pkgs; [
            hypervisorPackage
            gcc
            rust-toolchain
            rust-analyzer
            cargo-outdated
            cargo-udeps
            cargo-audit
            cargo-watch
            formatter
            treefmt
          ];
          git.hooks.enable = true;
          git.hooks.pre-commit.text = ''
            check-format
            check-clippy
            test-unit
          '';
          commands = [
            {
              name = "check-format";
              command = "treefmt --fail-on-change";
              help = "Check syntax";
              category = "check";
            }
            {
              name = "check-clippy";
              command = ''
                cargo clippy --all-targets --all-features
              '';
              help = "Run clippy and fail on warnings";
              category = "check";
            }
            {
              name = "check-flake";
              command = "nix flake check";
              help = "Check flake";
              category = "check";
            }
            {
              name = "check-udeps";
              command = ''
                PATH=${fenix.packages.${system}.latest.rustc}/bin:$PATH
                cargo udeps $@
              '';
              help = pkgs.cargo-udeps.meta.description;
              category = "check";
            }
            {
              name = "build-doc";
              command = ''
                cd $PRJ_ROOT
                cargo doc
              '';
              help =
                "Verify that the documentation builds without problems";
              category = "build";
            }
            {
              name = "build-network-partition";
              command = ''
                cargo build -p network-partition --release --target x86_64-unknown-linux-musl
              '';
              help = "Build network partition";
              category = "build";
            }
            {
              name = "build-network-partition-linux";
              command = ''
                cargo build -p network-partition-linux --release --target x86_64-unknown-linux-musl
              '';
              help = "Build linux network partition";
              category = "build";
            }
            {
              name = "build-echo";
              command = ''
                cargo build -p network-partition-linux --release --target x86_64-unknown-linux-musl
              '';
              help = "Build echo partition";
              category = "build";
            }
            {
              name = "build-no_std";
              command = ''
                cd $PRJ_ROOT
                cargo build -p network-partition --release --target thumbv6m-none-eabi
              '';
              help = "Verify that the library builds for no_std without std-features";
              category = "build";
            }

            {
              name = "test-unit";
              command = ''
                cd $PRJ_ROOT
                cargo test
              '';
              help = "Run unit tests";
              category = "test";
            }
            {
              name = "test-run-echo";
              command = ''
                cargo build -p network-partition-linux --release --target x86_64-unknown-linux-musl
                cargo build -p echo --release --target x86_64-unknown-linux-musl
                RUST_LOG=''${RUST_LOG:=trace} systemd-run --user --scope -- linux-apex-hypervisor --duration 10s config/hypervisor_config.yml
              '';
              help = "Run echo example using systemd scope and exit after 10 seconds";
              category = "test";
            }
            #####
            {
              name = "run-echo-scoped";
              command = ''
                RUST_LOG=''${RUST_LOG:=trace} systemd-run --user --scope -- linux-apex-hypervisor config/hypervisor_config.yml
              '';
              help = "Run echo example using systemd scope";
              category = "run";
            }
            {
              name = "check-outdated";
              command = "cargo-outdated outdated";
              help = pkgs.cargo-outdated.meta.description;
              category = "dev";
            }

            {
              name = "format";
              command = "treefmt";
              help = "Reformat";
              category = "dev";
            }
          ];
        };

        checks = {
          nixpkgs-fmt = pkgs.runCommand "check-format-nix"
            {
              nativeBuildInputs = [ formatter ];
            } "nixpkgs-fmt --check ${./.} && touch $out";
          cargo-fmt = pkgs.runCommand "check-format-rust"
            {
              nativeBuildInputs = [ rust-toolchain ];
            } "cd ${./.} && cargo fmt --check && touch $out";
        };

        packages = {
          network-partition = naerskLib.buildPackage rec {
            pname = "network-partition";
            root = ./.;
            cargoBuildOptions = x: x ++ [ "-p" pname ];
            cargoTestOptions = x: x ++ [ "-p" pname ];
          };
          network-partition-linux = naerskLib.buildPackage rec {
            pname = "network-partition-linux";
            root = ./.;
            cargoBuildOptions = x: x ++ [ "-p" pname ];
            cargoTestOptions = x: x ++ [ "-p" pname ];
          };
        };
      });
}
