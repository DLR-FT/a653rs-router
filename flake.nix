{
  description = "network-partition";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-22.11";

    utils.url = "github:numtide/flake-utils";

    devshell = {
      url = "github:numtide/devshell";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "utils";
    };

    fenix = {
      url = "github:nix-community/fenix";
      #inputs.nixpkgs.follows = "nixpkgs";
    };

    naersk = {
      url = "github:nix-community/naersk";
      #inputs.nixpkgs.follows = "nixpkgs";
    };

    hypervisor = {
      url = "github:dadada/apex-linux/stable-for-master-thesis";
      inputs.utils.follows = "utils";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.fenix.follows = "fenix";
      inputs.naersk.follows = "naersk";
    };

    xng-utils = {
      url = "github:dadada/xng-flake-utils/dev/dadada";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    fpga-project = {
      url = "git+ssh://git@github.com/dadada/vivado-coraz7-uart.git?ref=main";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "utils";
      inputs.xilinx-flake-utils.follows = "xilinx-flake-utils";
    };

    xilinx-flake-utils = {
      url = "github:aeronautical-informatics/xilinx-flake-utils/dev/add-devshell";
      # do not override any inputs here to not have to rebuild Xilinx Vitis
    };
  };

  outputs = { self, nixpkgs, utils, devshell, fenix, hypervisor, naersk, xng-utils, fpga-project, xilinx-flake-utils, ... }@inputs:
    utils.lib.eachSystem [ "x86_64-linux" ] (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            devshell.overlay
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
        xng-sys-img-local_echo = self.packages.${system}.xng-sys-image-local_ecxho;
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
              name = "test-run-echo";
              command = ''
                cargo build --release --target x86_64-unknown-linux-musl
                RUST_LOG=''${RUST_LOG:=trace} linux-apex-hypervisor --duration 10s config/linux/hv-client.yml 2> hv-client.log & \
                RUST_LOG=''${RUST_LOG:=trace} linux-apex-hypervisor --duration 10s config/linux/hv-server.yml 2> hv-server.log
              '';
              help = "Run echo example using systemd scope and exit after 10 seconds";
            }
            {
              name = "run-echo-scoped";
              command = ''
                RUST_LOG=''${RUST_LOG:=trace} systemd-run --user --scope -- linux-apex-hypervisor hv-client.yml
              '';
              help = "Run echo example using systemd scope";
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
                name = "jtag-boot";
                help = "Boot the network partition using JTAG";
                command = ''
                  nix --offline build .\#xng-sys-img-local_echo --print-build-logs
                  dir="$(mktemp -d)"
                  cp ${fpga} $dir/hw_export.xsa
                  unzip "$dir/hw_export.xsa" -d $dir
                  for cable in "210370AD523FA"
                  do 
                    xsct \
                      ${zynq7000Init} \
                      $dir/ps7_init.tcl \
                      $dir/hw_export.bit \
                      $dir/hw_export.xsa \
                      ${xng-sys-img-local_echo}/sys_img.elf \
                      "$cable" \
                      || printf "Failed to flash target"
                  done
                  rm -f "$dir/hw_export.xsa"
                  rm -r "$dir"
                '';
              }
              {
                name = "launch-picocom";
                help = "Launches picocom";
                command = ''
                  picocom --imap lfcrlf --baud 115200 ''${1:-/dev/ttyUSB1}
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
            inherit nixpkgs system;
            pkgs = nixpkgs.legacyPackages.${system};
            linux-apex-hypervisor = hypervisorPackage;
            network-partition-echo = self.packages.${system}.echo;
            echo-partition = self.packages.${system}.echo;
          };
        };
        packages = {
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
          np-zynq7000-remote_echo = naerskLib.buildPackage rec {
            pname = "np-zynq7000";
            CONFIG_DIR = ./config/remote_echo;
            root = ./.;
            cargoBuildOptions = x: x ++ [ "-p" pname "--target" "armv7a-none-eabi" ];
            doCheck = false;
            copyLibs = true;
            copyBins = false;
            #doDoc = true;
          };
          echo-client-zynq7000 = naerskLib.buildPackage rec {
            pname = "echo-client-zynq7000";
            CONFIG_DIR = ./config;
            root = ./.;
            cargoBuildOptions = x: x ++ [ "-p" pname "--target" "armv7a-none-eabi" ];
            doCheck = false;
            copyLibs = true;
            copyBins = false;
            #doDoc = true;
          };
          echo-server-zynq7000 = naerskLib.buildPackage rec {
            pname = "echo-server-zynq7000";
            CONFIG_DIR = ./config;
            root = ./.;
            cargoBuildOptions = x: x ++ [ "-p" pname "--target" "armv7a-none-eabi" ];
            doCheck = false;
            copyLibs = true;
            copyBins = false;
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
          xng-sys-img-local_echo = xng-utils.lib.buildXngSysImage {
            inherit pkgs;
            # TODO enable when armv7a-none-eabihf is in rust nightly or define target file
            hardFp = false;
            xngOps = self.packages.${system}.xng-ops;
            lithOsOps = self.packages.${system}.lithos-ops;
            xcf = pkgs.runCommandNoCC "patch-src" { } ''
              cp -r ${./. + "/config/local_echo/xml"} $out/
              #for file in $(find $out -name hypervisor.xml)
              #do
              #  substituteInPlace "$file" --replace 'baseAddr="0xE0001000"' 'baseAddr="0xE0000000"'
              #done
            '';
            name = "network_partition_local_echo";
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
        };
      });
}
