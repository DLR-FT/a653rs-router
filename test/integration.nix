{ hostPkgs
, configurator-client
, configurator-server
, echo-client
, echo-server
, hypervisor
, router-client
, router-server
, ...
}:
let
  hvName = "a653rs-linux-hypervisor";
  hv = "${hypervisor}/bin/${hvName}";
  environment = {
    RUST_BACKTRACE = "1";
    RUST_LOG = "debug";
  };
in
{
  name = "network-partition-integration";
  hostPkgs = hostPkgs;
  node.specialArgs = {
    inherit configurator-client configurator-server hypervisor echo-client echo-server router-client router-server;
  };

  nodes.client = { config, lib, pkgs, specialArgs, ... }: {
    environment.systemPackages = [ pkgs.tcpdump ];

    networking.firewall.enable = false;
    networking.interfaces.eth1.ipv4 = {
      addresses = [
        {
          address = "192.168.1.1";
          prefixLength = 24;
        }
      ];
    };

    environment.etc."hypervisor_config_client.yml" =
      {
        text = ''
          major_frame: 1s
          partitions:
            - id: 1
              name: Echo
              duration: 300ms
              offset: 0ms
              period: 1s
              image: ${specialArgs.echo-client}/bin/echo-sampling-linux
            - id: 2
              name: Network
              duration: 300ms
              offset: 350ms
              period: 1s
              image: ${specialArgs.router-client}/bin/router-echo-linux
              udp_ports:
                - "0.0.0.0:8081"
            - id: 3
              name: Cfgr
              duration: 300ms
              offset: 650ms
              period: 1s
              image: ${specialArgs.configurator-client}/bin/configurator--
          channel:
            - !Sampling
              name: EchoRequest
              msg_size: 1KB
              source: Echo
              destination:
                - Network
            - !Sampling
              name: EchoReply
              msg_size: 1KB
              source: Network
              destination:
                - Echo
            - !Sampling
              name: RouterConfig
              msg_size: 1KB
              source: Cfgr
              destination:
                - Network
        '';
        mode = "0444";
      };

    systemd.services.linux-hypervisor = {
      inherit environment;
      enable = true;
      description = "Echo client";
      unitConfig.Type = "simple";
      serviceConfig.ExecStart = "${hv} /etc/hypervisor_config_client.yml";
      wantedBy = [ "multi-user.target" ];
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];
    };
  };

  nodes.server = { config, lib, pkgs, specialArgs, ... }: {
    environment.systemPackages = [ pkgs.tcpdump ];

    systemd.services.linux-hypervisor = {
      inherit environment;
      enable = true;
      description = "Echo server";
      unitConfig.Type = "simple";
      serviceConfig.ExecStart = "${hv} /etc/hypervisor_config_server.yml";
      wantedBy = [ "multi-user.target" ];
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];
    };

    networking.firewall.enable = false;
    networking.interfaces.eth1.ipv4 = {
      addresses = [
        {
          address = "192.168.1.2";
          prefixLength = 24;
        }
      ];
    };

    environment.etc."hypervisor_config_server.yml" = {
      text = ''
        major_frame: 1s
        partitions:
          - id: 0
            name: Echo
            duration: 300ms
            offset: 0ms
            period: 1s
            image: ${specialArgs.echo-server}/bin/echo-sampling-linux
          - id: 1
            name: Network
            duration: 300ms
            offset: 400ms
            period: 1s
            image: ${specialArgs.router-server}/bin/router-echo-linux
            udp_ports:
              - "0.0.0.0:8082"
          - id: 3
            name: Cfgr
            duration: 300ms
            offset: 700ms
            period: 1s
            image: ${specialArgs.configurator-server}/bin/configurator--
        channel:
          - !Sampling
            name: EchoRequest
            msg_size: 1KB
            source: Network
            destination:
              - Echo
          - !Sampling
            name: EchoReply
            msg_size: 1KB
            source: Echo
            destination:
              - Network
          - !Sampling
            name: RouterConfig
            msg_size: 1KB
            source: Cfgr
            destination:
              - Network

      '';
      mode = "0444";
    };
  };

  testScript = ''
    server.wait_for_unit("linux-hypervisor.service")
    client.wait_for_unit("linux-hypervisor.service")
    client.wait_for_console_text("EchoRequest: seqnr = 10")
    _status, out = client.execute("journalctl -u linux-hypervisor.service")
    if not "EchoReply: seqnr =" in out:
        raise Exception("Expected to see an echo reply by now")
  '';
}
