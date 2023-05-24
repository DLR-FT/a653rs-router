{
  description = "network-partition";

  inputs = {
    utils.url = "github:numtide/flake-utils";

    devshell = {
      url = "github:numtide/devshell";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "utils";
    };

    fenix = {
      url = "github:nix-community/fenix";
    };

    naersk = {
      url = "github:nix-community/naersk";
    };

    hypervisor = {
      url = "github:dadada/apex-linux/udp-network-driver";
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
    trusted-substituters = "https://cache.ft-ssy-stonks.intra.dlr.de";
    substituters = "https://cache.ft-ssy-stonks.intra.dlr.de";
    trusted-public-keys = "ft-ssy-stonks.intra.dlr.de:xWBi+hGpebqGVgcYJtcPyW4BXBQ6TmI15c5OHf6htpM=";
  };

  outputs = { self, nixpkgs, utils, devshell, fenix, hypervisor, naersk, xng-utils, fpga-project, xilinx-flake-utils, ... }@inputs:
    utils.lib.eachSystem [ "x86_64-linux" ] (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            devshell.overlays.default
          ];
        };
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
        naerskLib = (naersk.lib.${system}.override {
          cargo = rust-toolchain;
          rustc = rust-toolchain;
        });
        hypervisorPackage = hypervisor.packages.${system}.linux-apex-hypervisor;

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
        xng-sys-img-local_echo = self.packages.${system}.xng-sys-img-local_echo;
      in
      {
        inherit formatter;

        # TODO merge into default dev shell
        devShells.xng =
          let
            mkShell = pkgs.mkShell.override { stdenv = pkgs.gccMultiStdenv; };
          in
          with self.packages."${system}"; mkShell {
            C_INCLUDE_PATH = "${xng-ops}/include";
            inputsFrom = [ xng-sys-img-local_echo ];
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

        devShells.default = pkgs.devshell.mkShell {
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
              name = "build-no_std";
              command = ''
                cd $PRJ_ROOT
                cargo build -p network-partition --release --target thumbv7m-none-eabi
              '';
              help = "Verify that the library builds for no_std without std-features";
            }
            {
              name = "test-run-echo-linux";
              command = ''
                cargo build --release --target x86_64-unknown-linux-musl
                RUST_LOG=''${RUST_LOG:=info} linux-apex-hypervisor --duration 10s config/linux/hv-server.yml 2> hv-server.log & \
                RUST_LOG=''${RUST_LOG:=info} linux-apex-hypervisor --duration 10s config/linux/hv-client.yml 2> hv-client.log &
              '';
              help = "Run echo example using existing scope and exit after 10 seconds";
            }
            {
              name = "test-run-echo-scoped";
              command = ''
                cargo build --release --target x86_64-unknown-linux-musl
                RUST_LOG=''${RUST_LOG:=info} systemd-run --user --scope -- linux-apex-hypervisor --duration 10s config/linux/hv-server.yml & \
                RUST_LOG=''${RUST_LOG:=info} systemd-run --user --scope -- linux-apex-hypervisor --duration 10s config/linux/hv-client.yml
              '';
              help = "Run echo example using apex-linux inside systemd scope and exit after 10 seconds";
            }
            {
              name = "test-run-echo-cora";
              command = ''
                nix develop .#deploy -c flash-echo-server && nix develop .#deploy -c flash-echo-client
              '';
              help = "Run echo example on two CoraZ7";
            }
            {
              name = "test-run-throughput-local";
              command = ''
                nix develop .#deploy -c flash-throughput-local
              '';
              help = "Run local (through IO-partition) throughput example on CoraZ7";
            }
            {
              name = "test-run-throughput-direct";
              command = ''
                nix develop .#deploy -c flash-throughput-direct
              '';
              help = "Run direct throughput example on CoraZ7";
            }
            {
              name = "test-run-throughput-remote";
              command = ''
                nix develop .#deploy -c flash-throughput-sink
                nix develop .#deploy -c flash-throughput-source
              '';
              help = "Run remote (through network) throughput example on CoraZ7";
            }
            {
              name = "test-run-echo-local";
              command = ''
                nix develop .#deploy -c flash-echo-local
              '';
              help = "Run local echo test on CoraZ7";
            }
            {
              name = "test-run-echo-direct";
              command = ''
                nix develop .#deploy -c flash-echo-direct
              '';
              help = "Run direct echo test on CoraZ7";
            }
          ];
        };

        # Separate devshell so we do not need to build Vitis if some flake input does not match just for changing code.
        devShells.deploy =
          let
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
                name = "run-local_echo";
                help = "Boot the network partition using JTAG";
                command = ''
                  dir="outputs/local_echo"
                  mkdir -p "$dir"
                  swdir="$dir/img"

                  nix build .#xng-sys-img-local_echo -o $swdir

                  hwdir="$dir/hardware"
                  mkdir -p "$hwdir"
                  cp --no-preserve=all ${fpga} $hwdir/hw_export.xsa
                  unzip -u "$hwdir/hw_export.xsa" -d "$hwdir"

                  for cable in "210370AD523FA"
                  do 
                    xsct \
                      ${zynq7000Init} \
                      $hwdir/ps7_init.tcl \
                      $hwdir/hw_export.bit \
                      $hwdir/hw_export.xsa \
                      $swdir/sys_img.elf \
                      "$cable" \
                      || printf "Failed to flash target"
                  done
                '';
              }
              {
                name = "flash";
                help = "Compile and flash a configuration";
                command = ''
                  example="''${1:-local_echo}"
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
                name = "flash-echo-direct";
                command = "flash echo_direct 210370AD523FA";
              }
              {
                name = "flash-echo-local";
                command = "flash local_echo 210370AD523FA";
              }
              {
                name = "flash-echo-client";
                command = "flash echo_client 210370AD523FA";
              }
              {
                name = "flash-echo-server";
                command = "flash echo_server 210370AD5202A";
              }
              {
                name = "flash-throughput-local";
                command = "flash throughput-local 210370AD523FA";
              }
              {
                name = "flash-throughput-direct";
                command = "flash throughput-direct 210370AD523FA";
              }
              {
                name = "flash-throughput-source";
                command = "flash throughput-source 210370AD523FA";
              }
              {
                name = "flash-throughput-sink";
                command = "flash throughput-sink 210370AD5202A";
              }
              {
                name = "launch-picocom";
                help = "Launches picocom";
                command = ''
                  picocom --imap lfcrlf --baud 115200 ''${1:-/dev/ttyUSB1} ''${@}
                '';
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
          run-echo-with-timeout = import ./test/integration.nix {
            inherit nixpkgs pkgs system;
            linux-apex-hypervisor = hypervisorPackage;
            echo-linux = self.packages.${system}.echo-linux;
          };
        };
        packages = {
          all-images = nixpkgs.legacyPackages.${system}.linkFarmFromDrvs "all-images" (
            with self.packages.${system}; [
              xng-sys-img-echo_client
              xng-sys-img-echo_direct
              xng-sys-img-echo_server
              xng-sys-img-local_echo
              xng-sys-img-throughput-direct
              xng-sys-img-throughput-local
              xng-sys-img-throughput-sink
              xng-sys-img-throughput-source
            ]
          );
          throughput-local-np = naerskLib.buildPackage rec {
            pname = "throughput-local-np";
            CONFIG_DIR = ./config/throughput/local;
            root = ./.;
            cargoBuildOptions = x: x ++ [ "-p" "np-zynq7000" "--target" "armv7a-none-eabi" ];
            doCheck = false;
            copyLibs = true;
            copyBins = false;
            #doDoc = true;
          };
          throughput-remote-source-np = naerskLib.buildPackage rec {
            pname = "throughput-local-np";
            CONFIG_DIR = ./config/throughput/remote/source;
            root = ./.;
            cargoBuildOptions = x: x ++ [ "-p" "np-zynq7000" "--target" "armv7a-none-eabi" ];
            doCheck = false;
            copyLibs = true;
            copyBins = false;
            #doDoc = true;
          };
          throughput-remote-sink-np = naerskLib.buildPackage rec {
            pname = "throughput-local-np";
            CONFIG_DIR = ./config/throughput/remote/sink;
            root = ./.;
            cargoBuildOptions = x: x ++ [ "-p" "np-zynq7000" "--target" "armv7a-none-eabi" ];
            doCheck = false;
            copyLibs = true;
            copyBins = false;
            #doDoc = true;
          };
          throughput-source = naerskLib.buildPackage rec {
            pname = "thoughput-source";
            root = ./.;
            cargoBuildOptions = x: x ++ [ "-p" "throughput-zynq7000" "--target" "armv7a-none-eabi" "--features" "sender" ];
            doCheck = false;
            copyLibs = true;
            copyBins = false;
            #doDoc = true;
          };
          throughput-sink = naerskLib.buildPackage rec {
            pname = "thoughput-sink";
            root = ./.;
            cargoBuildOptions = x: x ++ [ "-p" "throughput-zynq7000" "--target" "armv7a-none-eabi" "--features" "receiver" ];
            doCheck = false;
            copyLibs = true;
            copyBins = false;
            #doDoc = true;
          };
          echo = naerskLib.buildPackage rec {
            pname = "echo";
            CONFIG_DIR = ./config;
            root = ./.;
            cargoBuildOptions = x: x ++ [ "-p" pname "--target" "x86_64-unknown-linux-musl" ];
            cargoTestOptions = x: x ++ [ "-p" pname "--target" "x86_64-unknown-linux-musl" ];
            doCheck = true;
            #doDoc = true;
          };
          np-zynq7000-local_echo = naerskLib.buildPackage rec {
            pname = "np-zynq7000";
            CONFIG_DIR = ./config/local_echo;
            root = ./.;
            cargoBuildOptions = x: x ++ [ "-p" pname "--target" "armv7a-none-eabi" ];
            doCheck = false;
            copyLibs = true;
            copyBins = false;
            #doDoc = true;
          };
          np-zynq7000-echo_client = naerskLib.buildPackage rec {
            pname = "np-zynq7000";
            CONFIG_DIR = ./config/echo_client;
            root = ./.;
            cargoBuildOptions = x: x ++ [ "-p" pname "--target" "armv7a-none-eabi" ];
            doCheck = false;
            copyLibs = true;
            copyBins = false;
            #doDoc = true;
          };
          np-zynq7000-echo_server = naerskLib.buildPackage rec {
            pname = "np-zynq7000";
            CONFIG_DIR = ./config/echo_server;
            root = ./.;
            cargoBuildOptions = x: x ++ [ "-p" pname "--target" "armv7a-none-eabi" ];
            doCheck = false;
            copyLibs = true;
            copyBins = false;
            #doDoc = true;
          };
          echo-client-zynq7000 = naerskLib.buildPackage rec {
            pname = "echo-client-zynq7000";
            root = ./.;
            cargoBuildOptions = x: x ++ [ "-p" pname "--target" "armv7a-none-eabi" ];
            doCheck = false;
            copyLibs = true;
            copyBins = false;
            #doDoc = true;
          };
          echo-server-zynq7000 = naerskLib.buildPackage rec {
            pname = "echo-server-zynq7000";
            root = ./.;
            cargoBuildOptions = x: x ++ [ "-p" pname "--target" "armv7a-none-eabi" ];
            doCheck = false;
            copyLibs = true;
            copyBins = false;
            #doDoc = true;
          };
          echo-linux = naerskLib.buildPackage rec {
            pname = "echo-linux";
            CONFIG_DIR = ./config/linux;
            root = ./.;
            cargoBuildOptions = x: x ++ [ "-p" pname "--target" "x86_64-unknown-linux-musl" ];
            cargoTestOptions = x: x ++ [ "-p" pname "--target" "x86_64-unknown-linux-musl" ];
            doCheck = true;
            #doDoc = true;
          };
          xng-ops = xng-utils.lib.buildXngOps {
            inherit pkgs;
            src = xngSrcs.xng;
          };
          lithos-ops = xng-utils.lib.buildLithOsOps {
            inherit pkgs;
            src = xngSrcs.lithos;
          };
          xng-sys-img-echo_direct = xng-utils.lib.buildXngSysImage {
            inherit pkgs;
            hardFp = false;
            xngOps = self.packages.${system}.xng-ops;
            lithOsOps = self.packages.${system}.lithos-ops;
            xcf = pkgs.runCommandNoCC "patch-src" { } ''
              cp -r ${./. + "/config/echo_direct/xml"} $out/
            '';
            name = "xng-sys-img-echo-direct";
            partitions = {
              EchoClient = {
                src = "${self.packages."${system}".echo-client-zynq7000}/lib/libecho_client_zynq7000.a";
                enableLithOs = true;
                ltcf = ./config/echo_direct/echo_client.ltcf;
              };
              EchoServer = {
                src = "${self.packages."${system}".echo-server-zynq7000}/lib/libecho_server_zynq7000.a";
                enableLithOs = true;
                ltcf = ./config/echo_direct/echo_server.ltcf;
              };
            };
          };
          xng-sys-img-local_echo = xng-utils.lib.buildXngSysImage {
            inherit pkgs;
            hardFp = false;
            xngOps = self.packages.${system}.xng-ops;
            lithOsOps = self.packages.${system}.lithos-ops;
            xcf = pkgs.runCommandNoCC "patch-src" { } ''
              cp -r ${./. + "/config/local_echo/xml"} $out/
            '';
            name = "xng-sys-img-echo-local";
            partitions = {
              NetworkPartition = {
                src = "${self.packages."${system}".np-zynq7000-local_echo}/lib/libnp_zynq7000.a";
                enableLithOs = true;
                ltcf = ./config/local_echo/network_partition.ltcf;
              };
              EchoClient = {
                src = "${self.packages."${system}".echo-client-zynq7000}/lib/libecho_client_zynq7000.a";
                enableLithOs = true;
                ltcf = ./config/local_echo/echo_client.ltcf;
              };
              EchoServer = {
                src = "${self.packages."${system}".echo-server-zynq7000}/lib/libecho_server_zynq7000.a";
                enableLithOs = true;
                ltcf = ./config/local_echo/echo_server.ltcf;
              };
            };
          };
          xng-sys-img-throughput-local = xng-utils.lib.buildXngSysImage {
            inherit pkgs;
            hardFp = false;
            xngOps = self.packages.${system}.xng-ops;
            lithOsOps = self.packages.${system}.lithos-ops;
            xcf = pkgs.runCommandNoCC "patch-src" { } ''
              cp -r ${./. + "/config/throughput/local/xml"} $out/
            '';
            name = "xng-sys-img-throughput-local";
            partitions = {
              NetworkPartition = {
                src = "${self.packages."${system}".throughput-local-np}/lib/libnp_zynq7000.a";
                enableLithOs = true;
                ltcf = ./config/throughput/local/network_partition.ltcf;
              };
              Source = {
                src = "${self.packages."${system}".throughput-source}/lib/libthroughput_zynq7000.a";
                enableLithOs = true;
                ltcf = ./config/throughput/local/source.ltcf;
              };
              Sink = {
                src = "${self.packages."${system}".throughput-sink}/lib/libthroughput_zynq7000.a";
                enableLithOs = true;
                ltcf = ./config/throughput/local/sink.ltcf;
              };
            };
          };
          xng-sys-img-throughput-direct = xng-utils.lib.buildXngSysImage {
            inherit pkgs;
            hardFp = false;
            xngOps = self.packages.${system}.xng-ops;
            lithOsOps = self.packages.${system}.lithos-ops;
            xcf = pkgs.runCommandNoCC "patch-src" { } ''
              cp -r ${./. + "/config/throughput/direct/xml"} $out/
            '';
            name = "xng-sys-img-throughput-direct";
            partitions = {
              Source = {
                src = "${self.packages."${system}".throughput-source}/lib/libthroughput_zynq7000.a";
                enableLithOs = true;
                ltcf = ./config/throughput/direct/source.ltcf;
              };
              Sink = {
                src = "${self.packages."${system}".throughput-sink}/lib/libthroughput_zynq7000.a";
                enableLithOs = true;
                ltcf = ./config/throughput/direct/sink.ltcf;
              };
            };
          };
          xng-sys-img-throughput-source = xng-utils.lib.buildXngSysImage {
            inherit pkgs;
            hardFp = false;
            xngOps = self.packages.${system}.xng-ops;
            lithOsOps = self.packages.${system}.lithos-ops;
            xcf = pkgs.runCommandNoCC "patch-src" { } ''
              cp -r ${./. + "/config/throughput/remote/source/xml"} $out/
            '';
            name = "xng-sys-img-throughput-source";
            partitions = {
              NetworkPartition = {
                src = "${self.packages."${system}".throughput-remote-source-np}/lib/libnp_zynq7000.a";
                enableLithOs = true;
                ltcf = ./config/throughput/remote/source/network_partition.ltcf;
              };
              Source = {
                src = "${self.packages."${system}".throughput-source}/lib/libthroughput_zynq7000.a";
                enableLithOs = true;
                ltcf = ./config/throughput/remote/source/source.ltcf;
              };
            };
          };
          xng-sys-img-throughput-sink = xng-utils.lib.buildXngSysImage {
            inherit pkgs;
            hardFp = false;
            xngOps = self.packages.${system}.xng-ops;
            lithOsOps = self.packages.${system}.lithos-ops;
            xcf = pkgs.runCommandNoCC "patch-src" { } ''
              cp -r ${./. + "/config/throughput/remote/sink/xml"} $out/
            '';
            name = "xng-sys-img-throughput-sink";
            partitions = {
              NetworkPartition = {
                src = "${self.packages."${system}".throughput-remote-sink-np}/lib/libnp_zynq7000.a";
                enableLithOs = true;
                ltcf = ./config/throughput/remote/sink/network_partition.ltcf;
              };
              Sink = {
                src = "${self.packages."${system}".throughput-sink}/lib/libthroughput_zynq7000.a";
                enableLithOs = true;
                ltcf = ./config/throughput/remote/sink/sink.ltcf;
              };
            };
          };
          xng-sys-img-echo_client = xng-utils.lib.buildXngSysImage {
            inherit pkgs;
            hardFp = false;
            xngOps = self.packages.${system}.xng-ops;
            lithOsOps = self.packages.${system}.lithos-ops;
            xcf = pkgs.runCommandNoCC "patch-src" { } ''
              cp -r ${./. + "/config/echo_client/xml"} $out/
            '';
            name = "xng-sys-img-echo-client";
            partitions = {
              NetworkPartition = {
                src = "${self.packages."${system}".np-zynq7000-echo_client}/lib/libnp_zynq7000.a";
                enableLithOs = true;
                ltcf = ./config/echo_client/network_partition.ltcf;
              };
              EchoClient = {
                src = "${self.packages."${system}".echo-client-zynq7000}/lib/libecho_client_zynq7000.a";
                enableLithOs = true;
                ltcf = ./config/echo_client/echo_client.ltcf;
              };
            };
          };
          xng-sys-img-echo_server = xng-utils.lib.buildXngSysImage {
            inherit pkgs;
            hardFp = false;
            xngOps = self.packages.${system}.xng-ops;
            lithOsOps = self.packages.${system}.lithos-ops;
            xcf = pkgs.runCommandNoCC "patch-src" { } ''
              cp -r ${./. + "/config/echo_server/xml"} $out/
            '';
            name = "xng-sys-img-echo-server";
            partitions = {
              NetworkPartition = {
                src = "${self.packages."${system}".np-zynq7000-echo_server}/lib/libnp_zynq7000.a";
                enableLithOs = true;
                ltcf = ./config/echo_server/network_partition.ltcf;
              };
              EchoServer = {
                src = "${self.packages."${system}".echo-server-zynq7000}/lib/libecho_server_zynq7000.a";
                enableLithOs = true;
                ltcf = ./config/echo_server/echo_server.ltcf;
              };
            };
          };
        };
      });
}
