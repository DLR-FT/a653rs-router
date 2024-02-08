{ pkgs, a653rs-linux-hypervisor, partitions, runTest, ... }:
let
  inherit (partitions) echo-linux a653rs-router-linux router-echo-server-linux;

  echo = echo-linux;
  hypervisor = a653rs-linux-hypervisor;
  router = a653rs-router-linux;
in
runTest (import ./run-hypervisor.nix {
  hostPkgs = pkgs;
  nodeA = {
    inherit echo hypervisor router;
    hypervisorConfig = ./client/hypervisor.yml;
    routerConfig = ./client/router.yml;
  };
  nodeB = {
    inherit echo hypervisor router;
    hypervisorConfig = ./server/hypervisor.yml;
    routerConfig = ./server/router.yml;
  };
})
