{ pkgs }:
pkgs.testers.nixosTest
  ({ pkgs, ... }: {
    name = "network-partition-integration";

    nodes.system1 = { config, lib, pkgs, ... }: {
      environment.systemPackages = [ pkgs.arinc653rs-linux-hypervisor ];
      environment.etc."hypervisor_config_client.yml" =
        {
          text = ''
            major_frame: 1s
            partitions:
              - id: 1
                name: Echo
                duration: 100ms
                offset: 0ms
                period: 1s
                image: ${pkgs.echo-linux}/bin/echo
              - id: 2
                name: Network
                duration: 100ms
                offset: 500ms
                period: 1s
                image: ${pkgs.echo-linux}/bin/np-client
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
            major_frame: 100ms
            partitions:
              - id: 0
                name: Echo
                duration: 50ms
                offset: 50ms
                period: 100ms
                image: ${pkgs.echo-linux}/bin/echo-server
              - id: 1
                name: Network
                duration: 50ms
                offset: 0s
                period: 100ms
                image: ${pkgs.echo-linux}/bin/np-server
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
      system1.succeed("a653rs-linux-hypervisor --duration 10s /etc/hypervisor_config_server.yml & linux-apex-hypervisor --duration 10s /etc/hypervisor_config_client.yml")
    '';
  })
