{
  description = "network-partition";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-22.11";

    utils.url = "github:numtide/flake-utils";

    devshell = {
      url = "github:numtide/devshell";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "utils";
    };

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    hypervisor = {
      #url = "github:DLR-FT/a653rs-linux";
      url = "github:dadada/apex-linux?ref=udp-network-driver";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    xng-utils = {
      url = "github:dadada/xng-flake-utils/dev/dadada";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    fpga-project = {
      url = "git+ssh://git@gitlab.dlr.de/projekt-resilienz/vivado-coraz7-uart.git?ref=main";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    xilinx-flake-utils = {
      url = "github:aeronautical-informatics/xilinx-flake-utils";
      # do not override any inputs here to not have to rebuild Xilinx Vitis
    };
  };

  nixConfig = {
    extra-trusted-substituters = "https://cache.ft-ssy-stonks.intra.dlr.de";
    extra-substituters = "https://cache.ft-ssy-stonks.intra.dlr.de";
    extra-trusted-public-keys = "ft-ssy-stonks.intra.dlr.de:xWBi+hGpebqGVgcYJtcPyW4BXBQ6TmI15c5OHf6htpM=";
  };

  outputs = { self, nixpkgs, utils, devshell, fenix, hypervisor, xng-utils, fpga-project, xilinx-flake-utils, ... }@inputs:
    utils.lib.eachSystem [ "x86_64-linux" ]
      (system:
        let
          pkgs = import nixpkgs { inherit system; };
          formatter = pkgs.nixpkgs-fmt;
          rust-toolchain = with fenix.packages.${system}; combine [
            latest.rustc
            latest.cargo
            latest.clippy
            latest.rustfmt
            latest.rust-src
            latest.rust-analyzer
            targets.x86_64-unknown-linux-musl.latest.rust-std
            targets.thumbv7m-none-eabi.latest.rust-std
            targets.armv7a-none-eabi.latest.rust-std
          ];

          hypervisorPackage = hypervisor.packages.${system}.a653rs-linux-hypervisor;

          xngSrcs = {
            xng = pkgs.requireFile {
              name = "14-033.094.ops+armv7a-vmsa-tz+zynq7000.r16736.tbz2";
              url = "http://fentiss.com";
              sha256 = "1gb0cq3mmmr2fqj49p4svx07h5ccs8v564awlsc56mfjhm6jg3n4";
            };
            lithos = pkgs.requireFile {
              name = "020.080.ops.r7919+xngsmp.tbz2";
              url = "https://fentiss.com";
              sha256 = "1b73d6x3galw3bhj5nac7ifgp15zrsyipn4imwknr24gp1l14sc8";
            };
          };
        in
        {
          inherit formatter;

          # TODO merge into default dev shell
          devShells.xng =
            let
              pkgs = import nixpkgs { inherit system; overlays = [ devshell.overlays.default ]; };
              mkShell = pkgs.mkShell.override { stdenv = pkgs.gccMultiStdenv; };
            in
            with self.packages."${system}"; mkShell {
              C_INCLUDE_PATH = "${xng-ops}/include";
              inputsFrom = [ ]; #xng-sys-img-local_echo ];
              packages = with pkgs; [
                formatter
                treefmt
                rust-toolchain
                cargo-outdated
                cargo-udeps
                cargo-audit
                cargo-watch
              ];
            };

          devShells.default =
            let
              pkgs = import nixpkgs { inherit system; overlays = [ devshell.overlays.default ]; };
            in
            pkgs.devshell.mkShell {
              imports = [ "${devshell}/extra/git/hooks.nix" ];
              name = "network-partition-devshell";
              packages = with pkgs; [
                hypervisorPackage
                gcc
                rust-toolchain
                cargo-outdated
                cargo-udeps
                cargo-audit
                cargo-watch
                formatter
                treefmt
              ];
              git.hooks.enable = true;
              git.hooks.pre-commit.text = ''
                treefmt --fail-on-change
              '';
              commands = [
                {
                  name = "run-echo-router-client-server";
                  command = ''
                    print "TODO"
                  '';
                  help = "Run the echo server and client integration test";
                }
              ];
            };

          # Separate devshell so we do not need to build Vitis if some flake input does not match just for changing code.
          devShells.deploy =
            let
              pkgs = import nixpkgs { inherit system; overlays = [ devshell.overlays.default ]; };
              fpga = fpga-project.packages."${system}".default;
              zynq7000Init = ./deployment/zynq7000_init_te0706.tcl;
              vitis = xilinx-flake-utils.packages.${system}.vitis-unified-software-platform-vitis_2019-2_1106_2127;
              xng-sys-img-local_echo = self.packages."${system}".xng-sys-img-local_echo;
            in
            pkgs.devshell.mkShell {
              name = "network-partition-deploy";
              packages = with pkgs; [
                vitis
                picocom
              ];
              commands = [
                {
                  name = "run-xng";
                  help = "Compile and flash a configuration";
                  command = ''
                    example="''${1}"
                    cable="''${2:-210370AD5202A}"
                    dir="outputs/$example"
                    mkdir -p "$dir"
                    swdir="$dir/img"

                    nix build ".#xng-sys-img-$example" -o "$swdir"

                    hwdir="$dir/hardware"
                    mkdir -p "$hwdir"
                    cp --no-preserve=all ${fpga} $hwdir/hw_export.xsa
                    unzip -u "$hwdir/hw_export.xsa" -d "$hwdir"

                    xsct \
                      ${zynq7000Init} \
                      $hwdir/ps7_init.tcl \
                      $hwdir/hw_export.bit \
                      $hwdir/hw_export.xsa \
                      $swdir/sys_img.elf \
                      "$cable" \
                      || printf "Failed to flash target"
                  '';
                }
                {
                  name = "run-picocom";
                  help = "Launches picocom";
                  command = ''
                    picocom --imap lfcrlf --baud 115200 ''${1:-/dev/ttyUSB1} ''${@}
                  '';
                }
              ];
            };

          checks =
            let
              nixos-lib = nixpkgs.lib.nixos;
            in
            with self.packages.${system}; {
              nixpkgs-fmt = pkgs.runCommand "check-format-nix"
                {
                  nativeBuildInputs = [ formatter ];
                } "nixpkgs-fmt --check ${./.} && touch $out";
              cargo-fmt = pkgs.runCommand "check-format-rust"
                {
                  nativeBuildInputs = [ rust-toolchain ];
                } "cd ${./.} && cargo fmt --check && touch $out";
              integration = nixos-lib.runTest (import ./test/integration.nix {
                hostPkgs = pkgs;
                configurator-client = configurator---client;
                configurator-server = configurator---server;
                echo-client = echo-sampling-linux-client;
                echo-server = echo-sampling-linux-server;
                hypervisor = hypervisorPackage;
                router-client = router-echo-linux-client;
                router-server = router-echo-linux-server;
              });
            };
          packages =
            let
              inherit (self.lib) allProducts;
              rustPlatform = (pkgs.makeRustPlatform { cargo = rust-toolchain; rustc = rust-toolchain; });
              platforms = [
                { feature = "dummy"; target = "x86_64-unknown-linux-gnu"; }
                { feature = "linux"; target = "x86_64-unknown-linux-musl"; }
                { feature = "xng"; target = "armv7a-none-eabi"; }
              ];
            in
            (allProducts {
              inherit rustPlatform platforms;
              products = [ "router" ];
              flavors = [ "client" "server" ];
              variants = [ "echo" "throughput" ];
            })
            //
            (allProducts {
              inherit rustPlatform platforms;
              products = [ "echo" ];
              flavors = [ "client" "server" ];
              variants = [ "sampling" "queuing" ];
            })
            //
            (allProducts {
              inherit rustPlatform;
              products = [ "configurator" ];
              flavors = [ "client" "server" ];
              variants = [ "" ];
              platforms = [
                { feature = ""; target = "x86_64-unknown-linux-musl"; }
              ];
            })
            //
            (allProducts {
              inherit rustPlatform;
              products = [ "throughput" ];
              flavors = [ "sender" "receiver" ];
              variants = [ "" ];
              platforms = [
                { feature = "xng"; target = "armv7a-none-eabi"; }
              ];
            })
            //
            {
              xng-ops = xng-utils.lib.buildXngOps {
                inherit pkgs;
                src = xngSrcs.xng;
              };
              lithos-ops = xng-utils.lib.buildLithOsOps {
                inherit pkgs;
                src = xngSrcs.lithos;
              };
            };
        }) // {
      lib = rec {
        mkExample = { rustPlatform, product, example, features, target }:
          rustPlatform.buildRustPackage {
            pname = example;
            version = "0.1.0";
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;
            cargoLock.outputHashes = {
              "a653rs-linux-0.2.0" = "sha256-PR+gpuqY9opGNdi9rmgjBKYoX827h0jLeRD+XXSOXXM=";
              "a653rs-postcard-0.2.0" = "sha256-xDM5PwV24ZQ3NPVl12A1zX7FvYgLUxcufMCft+BzOSU=";
              "a653rs-xng-0.1.0" = "sha256-7vZ8eWwLXzR4Fb4UCA2GyI8HRnKVR5NFcWumrzkUMNM=";
              "xng-rs-log-0.1.0" = "sha256-YIFFnjWsk6g9tQuRBqmPaXsY3s2+BpkAg5PCw2ZGYCU=";
            };
            buildPhase = ''
              cargo build --release --target "${target}" -p ${product} --example=${example} --features=${nixpkgs.lib.concatStringsSep "," features}
            '';
            doCheck = target != "armv7a-none-eabi";
            checkPhase = ''
              cargo test --target "${target}" -p ${product} --example=${example} --features=${nixpkgs.lib.concatStringsSep "," features} --frozen
            '';
            installPhase = ''
              mkdir -p "$out"/{bin,lib}
              if [[ "${target}" = "armv7a-none-eabi" ]]
              then
                cp "target/${target}"/release/examples/*.a "$out/lib"
              else
                cp "target/${target}/release/examples/${example}" "$out/bin"
              fi
            '';
          };
        allProducts = { rustPlatform, flavors, platforms, variants, products }: builtins.listToAttrs (
          map
            ({ product, flavor, platform, variant }:
              let
                example = "${product}-${variant}-${platform.feature}";
              in
              (nixpkgs.lib.nameValuePair
                "${example}-${flavor}"
                (mkExample {
                  inherit example product rustPlatform;
                  features = [ variant platform.feature flavor ];
                  target = platform.target;
                })
              )
            )
            (nixpkgs.lib.cartesianProductOfSets {
              "flavor" = flavors;
              "platform" = platforms;
              "variant" = variants;
              "product" = products;
            })
        );

      };
    };
}
