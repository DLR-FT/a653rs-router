{
  description = "network-partition";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-23.05";

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

          devShells.default =
            let
              pkgs = import nixpkgs { inherit system; overlays = [ devshell.overlays.default ]; };
              fpga = fpga-project.packages."${system}".default;
              zynq7000Init = ./deployment/zynq7000_init_te0706.tcl;
              vitis = xilinx-flake-utils.packages.${system}.vitis-unified-software-platform-vitis_2019-2_1106_2127;
            in
            pkgs.devshell.mkShell {
              imports = [ "${devshell}/extra/git/hooks.nix" ];
              name = "network-partition-devshell";
              packages = with pkgs; [
                cargo-audit
                cargo-llvm-cov
                cargo-nextest
                cargo-outdated
                cargo-udeps
                cargo-watch
                formatter
                gcc
                gitlab-clippy
                hypervisorPackage
                picocom
                rust-toolchain
                treefmt
                vitis
              ];
              git.hooks.enable = true;
              git.hooks.pre-commit.text = ''
                treefmt --fail-on-change
              '';
              commands = [
                {
                  name = "run-nixos-integration-test";
                  command = ''
                    nix build .#checks.${system}.integration
                  '';
                  help = "Run the echo server and client integration test";
                }
                {
                  name = "run-xng";
                  help = "Compile and flash a configuration. This command takes one argument, which is the name of the package in this flake output to run";
                  command = ''
                    if [ "$#" -lt 1 ]
                    then
                      printf "usage: run-xng <example> <cable-id>\n"
                      exit 1
                    fi
                    example="''${1}"
                    cable="''${2:-210370AD5202A}"
                    dir="outputs/$example"
                    mkdir -p "$dir"
                    swdir="$dir/img"

                    nix build ".#$example" -o "$swdir"

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
                  name = "run-echo-direct-xng";
                  help = "Compile, flash and run the echo client and server on XNG";
                  command = ''
                    run-xng "echo-direct-xng" 210370AD5202A
                  '';
                }
                {
                  name = "run-echo-local-xng";
                  help = "Compile, flash and run the echo client and server on XNG, with an itermediary router";
                  command = ''
                    run-xng "echo-local-xng" 210370AD5202A
                  '';
                }
                {
                  name = "run-echo-remote-xng";
                  help = "Compile, flash and run the echo client and server on XNG, on two distributed nodes";
                  command = ''
                    run-xng "echo-remote-xng-server" 210370AD5202A
                    run-xng "echo-remote-xng-client" 210370AD523FA
                  '';
                }
                {
                  name = "run-echo-alt-local-remote-xng";
                  help =
                    "Compile, flash and run the echo client and server on XNG, on two distributed nodes and locally";
                  command = ''
                    run-xng "echo-remote-xng-server" 210370AD5202A
                    run-xng "echo-alt-local-client-xng" 210370AD523FA
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
                configurator-client = configurator--linux-client;
                configurator-server = configurator--linux-server;
                echo-client = echo-sampling-linux-client;
                echo-server = echo-sampling-linux-server;
                hypervisor = hypervisorPackage;
                router-client = router-echo-linux-client;
                router-server = router-echo-linux-server;
              });
              xng-images = nixpkgs.legacyPackages.${system}.linkFarmFromDrvs "all-images" (
                with self.packages.${system}; [
                  echo-direct-xng
                  echo-local-xng
                  echo-remote-xng-client
                  echo-remote-xng-server
                  echo-alt-local-client-xng
                ]
              );
            };
          packages =
            let
              allProducts = self.lib.allProducts;
              xngImage = self.lib.xngImage;
              xngOps = self.packages.${system}.xng-ops;
              lithOsOps = self.packages.${system}.lithos-ops;
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
              flavors = [ "client" "server" "local" "alt-local-client" ];
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
              flavors = [ "client" "server" "local" "alt-local-client" ];
              variants = [ "" ];
              platforms = [
                { feature = "linux"; target = "x86_64-unknown-linux-musl"; }
                { feature = "xng"; target = "armv7a-none-eabi"; }
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
              echo-remote-xng-client = xngImage rec {
                inherit pkgs xngOps lithOsOps;
                name = "echo-remote-xng-client";
                partitions = {
                  Router = "${self.packages."${system}".router-echo-xng-client}/lib/librouter_echo_xng.a";
                  EchoClient = "${self.packages."${system}".echo-queuing-xng-client}/lib/libecho_queuing_xng.a";
                  Config = "${self.packages."${system}".configurator--xng-client}/lib/libconfigurator__xng.a";
                };
              };
              echo-remote-xng-server = xngImage rec {
                inherit pkgs xngOps lithOsOps;
                name = "echo-remote-xng-server";
                partitions = {
                  Router = "${self.packages."${system}".router-echo-xng-server}/lib/librouter_echo_xng.a";
                  EchoServer = "${self.packages."${system}".echo-queuing-xng-server}/lib/libecho_queuing_xng.a";
                  Config = "${self.packages."${system}".configurator--xng-server}/lib/libconfigurator__xng.a";
                };
              };
              echo-direct-xng = xngImage rec {
                inherit pkgs xngOps lithOsOps;
                name = "echo-direct-xng";
                partitions = {
                  EchoClient = "${self.packages."${system}".echo-queuing-xng-client}/lib/libecho_queuing_xng.a";
                  EchoServer = "${self.packages."${system}".echo-queuing-xng-server}/lib/libecho_queuing_xng.a";
                };
              };
              echo-local-xng = xngImage rec {
                inherit pkgs xngOps lithOsOps;
                name = "echo-local-xng";
                partitions = {
                  EchoClient = "${self.packages."${system}".echo-queuing-xng-client}/lib/libecho_queuing_xng.a";
                  EchoServer = "${self.packages."${system}".echo-queuing-xng-server}/lib/libecho_queuing_xng.a";
                  Router = "${self.packages."${system}".router-echo-xng-local}/lib/librouter_echo_xng.a";
                  Config = "${self.packages."${system}".configurator--xng-local}/lib/libconfigurator__xng.a";
                };
              };
              echo-alt-local-client-xng = xngImage rec {
                inherit pkgs xngOps lithOsOps;
                name = "echo-alt-local-client-xng";
                partitions = {
                  EchoClient = "${self.packages."${system}".echo-queuing-xng-client}/lib/libecho_queuing_xng.a";
                  EchoServer = "${self.packages."${system}".echo-queuing-xng-server}/lib/libecho_queuing_xng.a";
                  Router = "${self.packages."${system}".router-echo-xng-alt-local-client}/lib/librouter_echo_xng.a";
                  Config = "${self.packages."${system}".configurator--xng-alt-local-client}/lib/libconfigurator__xng.a";
                };
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
              "a653rs-linux-0.2.0" = "sha256-r7VzFSs+5Or2zclJD8gFlvCSDwqk8qutHvxbqyNhSPw=";
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
        xngImage =
          { pkgs
          , name
          , xngOps
          , lithOsOps
          , partitions
          }: xng-utils.lib.buildXngSysImage {
            inherit pkgs name xngOps lithOsOps;
            hardFp = false;
            xcf = pkgs.runCommandNoCC "patch-src" { } ''
              mkdir -p merged
              cp -r "${./config/shared}"/* "${./config/${name}/xml}"/* merged/
              cp -r merged $out
            '';
            partitions = pkgs.lib.concatMapAttrs
              (partName: value: {
                "${partName}" = {
                  src = value;
                  enableLithOs = true;
                  ltcf = ./config/shared/${nixpkgs.lib.toLower partName}.ltcf;
                };
              })
              partitions;
          };
      };
    };
}
