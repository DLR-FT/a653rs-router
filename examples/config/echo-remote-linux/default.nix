{ pkgs, a653rs-linux-hypervisor, partitions, runTest, ... }:
let
  inherit (partitions) configurator-linux echo-linux router-echo-client-linux router-echo-server-linux;

  configurator = configurator-linux;
  echo = echo-linux;
in
runTest (import ./run-hypervisor.nix {
  hostPkgs = pkgs;
  nodeA = {
    inherit configurator echo a653rs-linux-hypervisor;
    hypervisorConfig = ./node-a.yml;
    router = router-echo-client-linux;
    routeTable = ./node-a-route-table.json;
  };
  nodeB = {
    inherit configurator echo a653rs-linux-hypervisor;
    hypervisorConfig = ./node-b.yml;
    router = router-echo-server-linux;
    routeTable = ./node-b-route-table.json;
  };
})
