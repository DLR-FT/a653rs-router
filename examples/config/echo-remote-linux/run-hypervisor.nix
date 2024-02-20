{ hostPkgs
, nodeA # { a653rs-linux-hypervisor = hypervisor-package, hypervisorConfig = path-to-yaml-config, router = pkg, configurator = pkg, echo = pkg, routeTable = path-to-toml-config }
, nodeB # { a653rs-linux-hypervisor = hypervisor-package, hypervisorConfig, router, configurator, echo, routeTable }
, ...
}:
let
  hvName = "a653rs-linux-hypervisor";
  environment = {
    RUST_BACKTRACE = "1";
    RUST_LOG = "info";
  };
  mkNode = node: ipAddr: { config, lib, pkgs, ... }:
    assert (builtins.typeOf node) == "set";
    assert (builtins.typeOf ipAddr) == "string";
    let
      inherit (node) configurator echo a653rs-linux-hypervisor hypervisorConfig router routeTable;
    in
    {
      networking.firewall.enable = false;
      networking.interfaces.eth1.ipv4 = {
        addresses = [
          {
            address = ipAddr;
            prefixLength = 24;
          }
        ];
      };

      environment.etc."hypervisor_config.yml" = {
        source = hypervisorConfig;
        mode = "0444";
      };

      environment.etc."route-table.json" = {
        source = routeTable;
        mode = "0444";
      };

      systemd.services.linux-hypervisor = {
        inherit environment;
        enable = true;
        description = "Linux Hypervisor";
        unitConfig.Type = "simple";
        serviceConfig.ExecStart = "${a653rs-linux-hypervisor}/bin/a653rs-linux-hypervisor /etc/hypervisor_config.yml";
        wantedBy = [ "multi-user.target" ];
        after = [ "network-online.target" ];
        wants = [ "network-online.target" ];
        path = [
          a653rs-linux-hypervisor
          configurator
          echo
          router
        ];
      };
    };
in
{
  name = "a653rs-router-integration";
  hostPkgs = hostPkgs;
  nodes.nodeA = mkNode nodeA "192.168.1.1";
  nodes.nodeB = mkNode nodeB "192.168.1.2";

  testScript = ''
    nodeB.wait_for_unit("linux-hypervisor.service")
    nodeA.wait_for_unit("linux-hypervisor.service")
    nodeA.wait_for_console_text("EchoRequest: seqnr = 10")
    _status, out = nodeA.execute("journalctl -u linux-hypervisor.service")
    if not "EchoReply: seqnr =" in out:
        raise Exception("Expected to see an echo reply by now")
  '';
}
