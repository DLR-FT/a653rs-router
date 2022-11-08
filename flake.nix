{
  description = "network-partition";

  inputs = {
    utils.url = "github:numtide/flake-utils";

    devshell.url = "github:numtide/devshell";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    hypervisor.url = "git+ssh://git@github.com/aeronautical-informatics/apex-linux.git?ref=main";
  };

  outputs = { self, nixpkgs, utils, devshell, fenix, hypervisor, ... }@inputs:
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
        ];
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
          ];
          git.hooks.enable = true;
          git.hooks.pre-commit.text = "nix flake check";
          commands = [
            { package = "git-cliff"; }
            { package = "treefmt"; }
            {
              name = "verify-no_std";
              command = ''
                cd $PRJ_ROOT
                cargo build -p network-partition --release --target thumbv6m-none-eabi --features apex-rs/serde,apex-rs/strum
              '';
              help = "Verify that the library builds for no_std without std-features";
              category = "test";
            }
            {
              name = "udeps";
              command = ''
                PATH=${fenix.packages.${system}.latest.rustc}/bin:$PATH
                cargo udeps $@
              '';
              help = pkgs.cargo-udeps.meta.description;
            }
            {
              name = "outdated";
              command = "cargo-outdated outdated";
              help = pkgs.cargo-outdated.meta.description;
            }
            {
              name = "build";
              command = ''
                cargo build --release --target x86_64-unknown-linux-musl
              '';
              help = "Build network partition";
              category = "dev";
            }
            {
              name = "run";
              command = ''
                cargo build -p network-partition-linux --release --target x86_64-unknown-linux-musl
                RUST_LOG=''${RUST_LOG:=trace} linux-apex-hypervisor linux/hypervisor_config.yaml
              '';
              help = "Build and run the network partition using the hypervisor";
              category = "dev";
            }
            {
              name = "run-scoped";
              command = "systemd-run --user --scope run";
              help = "Run hypervisor with networ partition using systemd scope";
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
      });
}
