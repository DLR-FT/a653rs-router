{ nixpkgs ? <nixpkgs>
, pkgs ? import <nixpkgs> { inherit system; config = { }; }
, system ? builtins.currentSystem
, linux-apex-hypervisor
, network-partition-linux
, echo-partition
,
} @args:

import "${nixpkgs}/nixos/tests/make-test-python.nix"
  ({ pkgs, ... }: {
    name = "network-partition-integration";

    nodes.system1 = { config, lib, ... }: {
      environment.systemPackages = [ linux-apex-hypervisor ];
      environment.etc."hypervisor_config.yml" =
        {
          text = ''
            major_frame: 10s
            partitions:
              - id: 0
                name: Echo
                duration: 100ms
                offset: 0ms
                period: 200ms
                image: ${echo-partition}/bin/echo
              - id: 1
                name: Network
                duration: 50ms
                offset: 100ms
                period: 200ms
                image: ${network-partition-linux}/bin/network-partition-linux
            channel:
              - !Sampling
                name: EchoRequest
                msg_size: 10KB
                source: Echo
                destination:
                  - Network
              - !Sampling
                name: EchoReply
                msg_size: 10KB
                source: Network
                destination:
                  - Echo
          '';
          mode = "0444";
        };
    };

    testScript = ''
      system1.wait_for_unit("multi-user.target")
      system1.succeed("RUST_LOG=trace linux-apex-hypervisor --duration 10s /etc/hypervisor_config.yml")
    '';
  })
  args
