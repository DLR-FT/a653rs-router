{ nixpkgs ? <nixpkgs>
, pkgs ? import <nixpkgs> { inherit system; config = { }; }
, system ? builtins.currentSystem
, linux-apex-hypervisor
, echo-linux
} @args:

import "${nixpkgs}/nixos/tests/make-test-python.nix"
  ({ pkgs, ... }: {
    name = "network-partition-integration";

    nodes.system1 = { config, lib, ... }: {
      environment.systemPackages = [ linux-apex-hypervisor ];
      environment.etc."hypervisor_config_client.yml" =
        {
          text = ''
            major_frame: 10s
            partitions:
              - id: 0
                name: Echo
                duration: 100ms
                offset: 0ms
                period: 1s
                image: ${echo-linux}/bin/echo
              - id: 1
                name: Network
                duration: 100ms
                offset: 100ms
                period: 200ms
                image: ${echo-linux}/bin/np-client
                udp_ports:
                  - "127.0.0.1:34254"
            channel:
              - !Sampling
                name: EchoRequest
                msg_size: 100B
                source: Echo
                destination:
                  - Network
              - !Sampling
                name: EchoReply
                msg_size: 100B
                source: Network
                destination:
                  - Echo
          '';
          mode = "0444";
        };
      environment.etc."hypervisor_config_server.yml" =
        {
          text = ''
            major_frame: 1s
            partitions:
              - id: 0
                name: Echo
                duration: 100ms
                offset: 0ms
                period: 200ms
                image: ${echo-linux}/echo-server
              - id: 1
                name: Network
                duration: 100ms
                offset: 100ms
                period: 200ms
                image: ${echo-linux}/np-server
                udp_ports:
                  - "127.0.0.1:34256"
            channel:
              - !Sampling
                name: EchoRequest
                msg_size: 100B
                source: Network
                destination:
                  - Echo
              - !Sampling
                name: EchoReply
                msg_size: 100B
                source: Echo
                destination:
                  - Network
          '';
          mode = "0444";
        };
    };

    testScript = ''
      system1.wait_for_unit("multi-user.target")
      system1.succeed("RUST_LOG=trace linux-apex-hypervisor --duration 10s /etc/hypervisor_config_server.yml & RUST_LOG=trace linux-apex-hypervisor --duration 10s /etc/hypervisor_config_client.yml")
    '';
  })
  args
