{
  description = "a653rs-router";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-23.11";

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

    fpga-project = {
      url = "git+ssh://git@gitlab.dlr.de/projekt-resilienz/vivado-coraz7-uart.git?ref=main";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "utils";
    };

    xilinx-flake-utils = {
      follows = "fpga-project/xilinx-flake-utils";
      # do not override any inputs here to not have to rebuild Xilinx Vitis
    };
  };

  outputs = { self, nixpkgs, utils, devshell, fenix, hypervisor, xng-utils, fpga-project, xilinx-flake-utils, ... }@inputs:
    let
      cargoLock = {
        lockFile = ./Cargo.lock;
        outputHashes = {
          "a653rs-linux-0.2.0" = "sha256-K3fWsAooXrl/uOYRcjI2N3bonSyjc3oeeBBUTJ1X/0M=";
          "a653rs-postcard-0.2.0" = "sha256-xDM5PwV24ZQ3NPVl12A1zX7FvYgLUxcufMCft+BzOSU=";
          "a653rs-xng-0.1.0" = "sha256-7vZ8eWwLXzR4Fb4UCA2GyI8HRnKVR5NFcWumrzkUMNM=";
          "xng-rs-log-0.1.0" = "sha256-YIFFnjWsk6g9tQuRBqmPaXsY3s2+BpkAg5PCw2ZGYCU=";
        };
      };

    in
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
              zynq7000Init = ./a653rs-router-zynq7000/zynq7000_init_te0706.tcl;
              vitis = xilinx-flake-utils.packages.${system}.vitis-unified-software-platform-vitis_2019-2_1106_2127;
            in
            pkgs.devshell.mkShell {
              imports = [ "${devshell}/extra/git/hooks.nix" ];
              name = "a653rs-router-devshell";
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
                run-checks
              '';
              commands = [
                {
                  name = "run-checks";
                  command = ''
                    treefmt --fail-on-change
                    cargo check --all-features
                    cargo test --all-features
                    cargo doc --all-features
                  '';
                  help = "Run checks";
                }
                {
                  name = "run-nixos-integration-test";
                  command = ''
                    nix build .#checks.${system}.integration --print-build-logs
                  '';
                  help = "Run the echo server and client integration test";
                }
                {
                  name = "build-xng-images";
                  command = ''
                    nix build .#checks.${system}.xng-images --print-build-logs 
                  '';
                  help = "Build XNG images";
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
                {
                  name = "deploy-router-config-xng";
                  help = "Flashes a new configuration for the router using JTAG: deploy-router-config-xng <region> <cable> <cfg>";
                  command = ''
                    xsct ${./a653rs-router-zynq7000/flash-cfg.tcl} ''${@}
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
              integration =
                let
                  configurator = configurator-linux;
                  hypervisor = hypervisorPackage;
                in
                nixos-lib.runTest (import ./examples/config/echo-remote-linux {
                  hostPkgs = pkgs;
                  nodeA = {
                    inherit configurator hypervisor;
                    echo = echo-linux;
                    hypervisorConfig = ./examples/config/echo-remote-linux/node-a.yml;
                    router = router-echo-client-linux;
                    routeTable = ./examples/config/echo-remote-linux/node-a-route-table.json;
                  };
                  nodeB = {
                    inherit configurator hypervisor;
                    echo = echo-linux;
                    hypervisorConfig = ./examples/config/echo-remote-linux/node-b.yml;
                    router = router-echo-server-linux;
                    routeTable = ./examples/config/echo-remote-linux/node-b-route-table.json;
                  };
                });
              xng-images = nixpkgs.legacyPackages.${system}.linkFarmFromDrvs "all-images" (
                with self.packages.${system}; [
                  image-echo-direct-xng
                  image-echo-local-xng
                  image-echo-remote-xng-client
                  image-echo-remote-xng-server
                  image-echo-alt-local-client-xng
                ]
              );
            };

          packages =
            let
              mkExample = self.lib.mkExample;
              routerConfigBlob = name: {
                "0x16000000" = (pkgs.runCommandNoCC "router-config" { } ''
                  ${pkgs.lib.meta.getExe self.packages.${system}.a653rs-router-cfg} < ${./examples/config/${name}/route-table.json} > $out
                '').outPath;
              };
              xngOps = xng-utils.lib.buildXngOps {
                inherit pkgs;
                src = xngSrcs.xng;
              };
              lithOsOps = xng-utils.lib.buildLithOsOps {
                inherit pkgs;
                patches = [ ./patches/lithos-xng-armv7a-vmsa-tz.lds.patch ];
                src = xngSrcs.lithos;
              };
              xngImage = { name, partitions }: xng-utils.lib.buildXngSysImage {
                inherit pkgs name xngOps lithOsOps;
                extraBinaryBlobs = if (partitions ? "Router") then (routerConfigBlob name) else { };
                hardFp = false;
                xcf = pkgs.runCommandNoCC "patch-src" { } ''
                  mkdir -p merged
                  cp -r "${./examples/config/shared}"/* "${./examples/config/${name}/xml}"/* merged/
                  cp -r merged $out
                '';
                partitions = pkgs.lib.concatMapAttrs
                  (partName: value: {
                    "${partName}" = {
                      src = value;
                      enableLithOs = true;
                      forceXre = true;
                      ltcf = ./examples/config/shared/${nixpkgs.lib.toLower partName}.ltcf;
                    };
                  })
                  partitions;
              };
              rustPlatform = (pkgs.makeRustPlatform { cargo = rust-toolchain; rustc = rust-toolchain; });
            in
            rec
            {
              a653rs-router-cfg = rustPlatform.buildRustPackage {
                inherit cargoLock;
                pname = "a653rs-router-cfg";
                version = "0.1.0";
                src = ./.;
                doCheck = true;
              };

              configurator-linux = rustPlatform.buildRustPackage rec {
                inherit cargoLock;
                pname = "configurator-linux";
                target = "x86_64-unknown-linux-musl";
                version = "0.1.0";
                src = ./.;
                buildPhase = ''
                  cargo build --release -p ${pname} --target ${target}
                '';
                checkPhase = ''
                  cargo test -p ${pname} --target ${target} --frozen
                '';
                doCheck = false;
                installPhase = ''
                  mkdir -p "$out"/bin
                  cp target/${target}/release/${pname} "$out/bin"
                '';
              };

              configurator-xng = rustPlatform.buildRustPackage rec {
                inherit cargoLock;
                pname = "configurator-xng";
                target = "armv7a-none-eabi";
                version = "0.1.0";
                src = ./.;
                buildPhase = ''
                  cargo build --release -p ${pname} --target ${target}
                '';
                checkPhase = ''
                  cargo test -p ${pname} --target ${target} --frozen
                '';
                doCheck = false;
                installPhase = ''
                  mkdir -p "$out"/lib
                  cp target/${target}/release/*.a "$out/lib"
                '';
              };

              echo-linux = rustPlatform.buildRustPackage rec {
                inherit cargoLock;
                target = "x86_64-unknown-linux-musl";
                pname = "echo-linux";
                version = "0.1.0";
                src = ./.;
                buildPhase = ''
                  cargo build --release --target ${target} -p ${pname}
                '';
                doCheck = false;
                installPhase = ''
                  mkdir -p "$out"/bin
                  cp target/${target}/release/echo "$out/bin"
                '';
              };

              echo-xng = rustPlatform.buildRustPackage rec {
                inherit cargoLock;
                target = "armv7a-none-eabi";
                pname = "echo-xng";
                version = "0.1.0";
                src = ./.;
                buildPhase = ''
                  cargo build --release --target ${target} -p ${pname}
                '';
                doCheck = false;
                installPhase = ''
                  mkdir -p "$out"/lib
                  cp "target/${target}"/release/*.a "$out/lib"
                '';
              };

              router-echo-client-linux = mkExample {
                inherit rustPlatform;
                example = "router-echo-client-linux";
                product = "router";
                features = [ "linux" ];
                target = "x86_64-unknown-linux-musl";
              };
              router-echo-server-linux = mkExample {
                inherit rustPlatform;
                example = "router-echo-server-linux";
                product = "router";
                features = [ "linux" ];
                target = "x86_64-unknown-linux-musl";
              };
              router-echo-client-xng = mkExample {
                inherit rustPlatform;
                example = "router-echo-client-xng";
                product = "router";
                features = [ "xng" ];
                target = "armv7a-none-eabi";
              };
              router-echo-server-xng = mkExample {
                inherit rustPlatform;
                example = "router-echo-server-xng";
                product = "router";
                features = [ "xng" ];
                target = "armv7a-none-eabi";
              };
              router-echo-local-xng = mkExample {
                inherit rustPlatform;
                example = "router-echo-local-xng";
                product = "router";
                features = [ "xng" ];
                target = "armv7a-none-eabi";
              };

              image-echo-remote-xng-client = xngImage {
                name = "echo-remote-xng-client";
                partitions = {
                  Router = "${router-echo-client-xng}/lib/librouter_echo_client_xng.a";
                  EchoClient = "${echo-xng}/lib/libecho_xng.a";
                  Config = "${configurator-xng}/lib/libconfigurator_xng.a";
                };
              };
              image-echo-remote-xng-server = xngImage {
                name = "echo-remote-xng-server";
                partitions = {
                  Router = "${router-echo-server-xng}/lib/librouter_echo_server_xng.a";
                  EchoServer = "${echo-xng}/lib/libecho_xng.a";
                  Config = "${configurator-xng}/lib/libconfigurator_xng.a";
                };
              };
              image-echo-direct-xng = xngImage {
                name = "echo-direct-xng";
                partitions = {
                  EchoClient = "${echo-xng}/lib/libecho_xng.a";
                  EchoServer = "${echo-xng}/lib/libecho_xng.a";
                };
              };
              image-echo-local-xng = xngImage {
                name = "echo-local-xng";
                partitions = {
                  EchoClient = "${echo-xng}/lib/libecho_xng.a";
                  EchoServer = "${echo-xng}/lib/libecho_xng.a";
                  Router = "${router-echo-local-xng}/lib/librouter_echo_local_xng.a";
                  Config = "${configurator-xng}/lib/libconfigurator_xng.a";
                };
              };
              image-echo-alt-local-client-xng = xngImage {
                name = "echo-alt-local-client-xng";
                partitions = {
                  EchoClient = "${echo-xng}/lib/libecho_xng.a";
                  EchoServer = "${echo-xng}/lib/libecho_xng.a";
                  Router = "${router-echo-client-xng}/lib/librouter_echo_client_xng.a";
                  Config = "${configurator-xng}/lib/libconfigurator_xng.a";
                };
              };
            };
        }
      ) //
    {
      lib = rec {
        mkExample = { rustPlatform, product, example, features, target }:
          rustPlatform.buildRustPackage {
            inherit cargoLock;
            pname = example;
            version = "0.1.0";
            src = ./.;
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
      };
    }; # outputs
}
