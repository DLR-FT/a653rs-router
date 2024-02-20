{ pkgs, a653rs-router-cfg, lithOsOps, partitions, xng-utils, xngOps, ... }:
let
  inherit (partitions) configurator-xng echo-xng router-echo-client-xng router-echo-server-xng router-echo-local-xng;

  routerConfigBlob = name: {
    "0x16000000" = (pkgs.runCommandNoCC "router-config" { } ''
      ${pkgs.lib.meta.getExe a653rs-router-cfg} < ${./examples/config/${name}/route-table.json} > $out
    '').outPath;
  };

  xngImage = { name, partitions }: xng-utils.lib.buildXngSysImage {
    inherit pkgs name xngOps lithOsOps;
    extraBinaryBlobs = if (partitions ? "Router") then (routerConfigBlob name) else { };
    hardFp = false;
    xcf = pkgs.runCommandNoCC "patch-src" { } ''
      mkdir -p merged
      cp -r "${./examples/config/shared}"/* "${./examples/config/${name}/xml}"/* merged/
      cp -r merged $out
    '';
    partitions = pkgs.lib.concatMapAttrs
      (partName: value: {
        "${partName}" = {
          src = value;
          enableLithOs = true;
          forceXre = true;
          ltcf = ./examples/config/shared/${pkgs.lib.toLower partName}.ltcf;
        };
      })
      partitions;
  };
in
rec {
  image-echo-remote-xng-client = xngImage {
    name = "echo-remote-xng-client";
    partitions = {
      Router = "${router-echo-client-xng}/lib/librouter_echo_client_xng.a";
      EchoClient = "${echo-xng}/lib/libecho_xng.a";
      Config = "${configurator-xng}/lib/libconfigurator_xng.a";
    };
  };
  image-echo-remote-xng-server = xngImage {
    name = "echo-remote-xng-server";
    partitions = {
      Router = "${router-echo-server-xng}/lib/librouter_echo_server_xng.a";
      EchoServer = "${echo-xng}/lib/libecho_xng.a";
      Config = "${configurator-xng}/lib/libconfigurator_xng.a";
    };
  };
  image-echo-direct-xng = xngImage {
    name = "echo-direct-xng";
    partitions = {
      EchoClient = "${echo-xng}/lib/libecho_xng.a";
      EchoServer = "${echo-xng}/lib/libecho_xng.a";
    };
  };
  image-echo-local-xng = xngImage {
    name = "echo-local-xng";
    partitions = {
      EchoClient = "${echo-xng}/lib/libecho_xng.a";
      EchoServer = "${echo-xng}/lib/libecho_xng.a";
      Router = "${router-echo-local-xng}/lib/librouter_echo_local_xng.a";
      Config = "${configurator-xng}/lib/libconfigurator_xng.a";
    };
  };
  image-echo-alt-local-client-xng = xngImage {
    name = "echo-alt-local-client-xng";
    partitions = {
      EchoClient = "${echo-xng}/lib/libecho_xng.a";
      EchoServer = "${echo-xng}/lib/libecho_xng.a";
      Router = "${router-echo-client-xng}/lib/librouter_echo_client_xng.a";
      Config = "${configurator-xng}/lib/libconfigurator_xng.a";
    };
  };
  xng-images = (pkgs.linkFarmFromDrvs "xng-images" [
    image-echo-direct-xng
    image-echo-local-xng
    image-echo-remote-xng-client
    image-echo-remote-xng-server
    image-echo-alt-local-client-xng
  ]);
}
