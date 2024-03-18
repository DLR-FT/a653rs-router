{ pkgs, rustToolchain, devshell, formatter, ... }:
pkgs.devshell.mkShell {
  imports = [ "${devshell}/extra/git/hooks.nix" ];
  name = "a653rs-router-devshell";
  packages = with pkgs; [
    cargo-audit
    cargo-bloat
    cargo-llvm-cov
    cargo-nextest
    cargo-outdated
    cargo-udeps
    cargo-watch
    formatter
    gcc
    gitlab-clippy
    rustToolchain
    treefmt
  ];
  git.hooks.enable = true;
  git.hooks.pre-commit.text = ''
    run-checks
  '';
  commands = [
    {
      name = "run-checks";
      command = ''
        set +x
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
        nix build .#checks.${pkgs.system}.integration --print-build-logs
      '';
      help = "Run the echo server and client integration test";
    }
    {
      name = "run-xng";
      help = "Compile and flash a configuration. This command takes two arguments, which is the name of the package in this flake output to run and the cable ID of the JTAG target";
      command = ''
        nix build "$PRJ_ROOT#image-$1" -o xng-image
        $PRJ_ROOT/a653rs-router-zynq7000/flash "$2" xng-image/sys_img.elf hw_export.xsa
      '';
    }
    {
      name = "run-echo-direct-xng";
      help = "Compile, flash and run the echo client and server on XNG";
      command = ''
        run-xng "echo-direct-xng" "''${1}"
      '';
    }
    {
      name = "run-echo-local-xng";
      help = "Compile, flash and run the echo client and server on XNG, with an intermediary router";
      command = ''
        run-xng "echo-local-xng" $@
      '';
    }
    {
      name = "run-echo-remote-xng";
      help = "Compile, flash and run the echo client and server on XNG, on two distributed nodes";
      command = ''
        run-xng "echo-remote-server-xng" "''${1}"
        run-xng "echo-remote-client-xng" "''${2}"
      '';
    }
    {
      name = "run-echo-remote-alt-xng";
      help =
        "Compile, flash and run the echo client and server on XNG, on two distributed nodes and locally";
      command = ''
        run-xng "echo-remote-server-xng" "''${1}"
        run-xng "echo-remote-client-alt-xng" "''${2}"
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
        xsct $PRJ_ROOT/a653rs-router-zynq7000/flash-cfg.tcl ''${@}
      '';
    }
    {
      name = "build-xng-images";
      help = "Build XNG images";
      command = ''
        nix build .#xng-images
      '';
    }
  ];
}
