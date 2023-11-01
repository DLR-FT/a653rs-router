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
    RUST_LOG = "info";
  };
in
{
  name = "a653rs-router-integration";
  hostPkgs = hostPkgs;

  nodes.client = { config, lib, pkgs, ... }: {

    networking.firewall.enable = false;
    networking.interfaces.eth1.ipv4 = {
      addresses = [
        {
          address = "192.168.1.1";
          prefixLength = 24;
        }
      ];
    };

    environment.etc."hypervisor_config_client.yml" = {
      source = ./client.yml;
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
      path = [
        configurator-client
        echo-client
        hypervisor
        router-client
      ];
    };
  };

  nodes.server = { config, lib, pkgs, specialArgs, ... }: {
    systemd.services.linux-hypervisor = {
      inherit environment;
      enable = true;
      description = "Echo server";
      unitConfig.Type = "simple";
      serviceConfig.ExecStart = "${hv} /etc/hypervisor_config_server.yml";
      wantedBy = [ "multi-user.target" ];
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];
      path = [
        configurator-server
        echo-server
        hypervisor
        router-server
      ];
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
      source = ./server.yml;
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
