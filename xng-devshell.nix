{ pkgs, fpga, vitis, ... }:
pkgs.devshell.mkShell {
  name = "a653rs-router-zynq7000-devshell";
  packages = with pkgs; [
    picocom
    vitis
  ];
  commands = [
    {
      name = "run-xng";
      help = "Compile and flash a configuration. This command takes one argument, which is the name of the package in this flake output to run";
      command = ''
        nix build "$PRJ_ROOT#$image-$1" -o xng-image
        $PRJ_ROOT/a653rs-router-zynq7000/flash "''${1:-210370AD5202A}" xng-image/sys_img.elf ${fpga}/hw_export.xsa
      '';
    }
    {
      name = "run-echo-direct-xng";
      help = "Compile, flash and run the echo client and server on XNG";
      command = ''
        run-xng "echo-direct-xng" "''${1:-210370AD5202A}"
      '';
    }
    {
      name = "run-echo-local-xng";
      help = "Compile, flash and run the echo client and server on XNG, with an intermediary router";
      command = ''
        run-xng "echo-local-xng" "''${1:-210370AD5202A}"
      '';
    }
    {
      name = "run-echo-remote-xng";
      help = "Compile, flash and run the echo client and server on XNG, on two distributed nodes";
      command = ''
        run-xng "echo-remote-xng-server" "''${1:-210370AD5202A}"
        run-xng "echo-remote-xng-client" "''${2:-210370AD523FA}"
      '';
    }
    {
      name = "run-echo-alt-local-remote-xng";
      help =
        "Compile, flash and run the echo client and server on XNG, on two distributed nodes and locally";
      command = ''
        run-xng "echo-remote-xng-server" "''${1:-210370AD5202A}"
        run-xng "echo-alt-local-client-xng" "''${2:-210370AD523FA}"
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
  ];
}
