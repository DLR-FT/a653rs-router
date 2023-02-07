{
  description = "network-partition";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/22.11";

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

    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
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
    };

    xilinx-workspace = {
      url = "git+ssh://git@gitlab.dlr.de/ft-ssy-aes/XANDAR/xilinx-workspace.git";
      flake = false;
    };

    xilinx-flake-utils = {
      url = "github:aeronautical-informatics/xilinx-flake-utils/dev/add-devshell";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "utils";
    };
  };

  outputs = { self, nixpkgs, utils, devshell, fenix, hypervisor, naersk, xng-utils, fpga-project, xilinx-workspace, xilinx-flake-utils, ... }@inputs:
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

        fpga = fpga-project.packages."${system}".default;
        # xsa = "${fpga}/hw_export.xsa";
        # bitstream = "${fpga}/hw_export.bit";
        # ps7Init = "${fpga}/ps7_init.tcl";
        zynq7000Init = "${xilinx-workspace}/deployment/scripts/tcl_lib/zynq7000_init_te0706.tcl";
        vitis = xilinx-flake-utils.packages.${system}.vitis-unified-software-platform-vitis_2019-2_1106_2127;
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
            inputsFrom = [ xng-sys-image ];
            packages = with pkgs; [
              formatter
              treefmt
              rust-toolchain
              rust-analyzer
              cargo-outdated
              cargo-udeps
              cargo-audit
              cargo-watch
            ];
          };

        devShells.default = pkgs.devshell.mkShell {
          imports = [ "${devshell}/extra/git/hooks.nix" ];
          name = "network-partition";
          env = [{ name = "UTILS_ROOT"; value = "../xilinx-workspace"; }];
          packages = with pkgs; [
            vitis
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
            treefmt --fail-on-change
            cargo clippy -p network-partition \
              -p network-partition-linux \
              -p network-partition \
              -p echo
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
                RUST_LOG=''${RUST_LOG:=trace} linux-apex-hypervisor --duration 10s config/hv-client.yml 2> hv-client.log & \
                RUST_LOG=''${RUST_LOG:=trace} linux-apex-hypervisor --duration 10s config/hv-server.yml 2> hv-server.log
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
            {
              name = "jtag-boot";
              help = "Boot the network partition using JTAG";
              command = ''
                mkdir -p xsa
                cp ${fpga} xsa/hw_export.xsa
                unzip xsa/hw_export.xsa -d xsa
                xsct \
                  ${zynq7000Init} \
                  xsa/ps7_init.tcl \
                  xsa/hw_export.bit \
                  xsa/hw_export.xsa \
                  result/sys_img.elf
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
          np-zynq7000 = naerskLib.buildPackage rec {
            pname = "np-zynq7000";
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
          xng-sys-image = xng-utils.lib.buildXngSysImage {
            inherit pkgs;
            # TODO enable when armv7a-none-eabihf is in rust nightly or define target file
            hardFp = false;
            xngOps = self.packages.${system}.xng-ops;
            lithOsOps = self.packages.${system}.lithos-ops;
            xcf = pkgs.runCommandNoCC "patch-src" { } ''
              cp -r ${./. + "/config/xml"} $out/
              #for file in $(find $out -name hypervisor.xml)
              #do
              #  substituteInPlace "$file" --replace 'baseAddr="0xE0001000"' 'baseAddr="0xE0000000"'
              #done
            '';
            name = "network_partition_example";
            partitions = {
              NetworkPartition = {
                src = "${self.packages."${system}".np-zynq7000}/lib/libnp_zynq7000.a";
                enableLithOs = true;
                ltcf = ./config/network_partition.ltcf;
              };
            };
          };
        };
      });
}
