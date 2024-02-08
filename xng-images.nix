{ pkgs, a653rs-router-cfg, lithOsOps, partitions, xng-utils, xngOps, ... }:
let
  inherit (partitions) echo-xng a653rs-router-zynq7000;

  routerConfigBlob = path: {
    "0x16000000" = (pkgs.runCommandNoCC "router-config" { } ''
      printf "Converting config at ${path}\n'"
      ${pkgs.lib.meta.getExe a653rs-router-cfg} < "${path}" > $out
    '').outPath;
  };

  xngImage = { name, partitions, path ? "${./examples/config/${name}}" }: xng-utils.lib.buildXngSysImage {
    inherit pkgs name xngOps lithOsOps;
    extraBinaryBlobs = if (partitions ? "Router") then (routerConfigBlob "${path}/router.yml") else { };
    hardFp = false;
    xcf = pkgs.runCommandNoCC "patch-src" { } ''
      mkdir -p merged
      cp -r "${./examples/config/shared}"/* "${path}"/xng/* merged/
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
  router = "${a653rs-router-zynq7000}/lib/librouter.a";
  echo = "${echo-xng}/lib/libecho_xng.a";
in
rec {
  image-echo-remote-client-xng = xngImage rec {
    name = "echo-remote";
    path = "${./examples/config/${name}/client}";
    partitions = {
      Router = router;
      EchoClient = echo;
    };
  };
  image-echo-remote-server-xng = xngImage rec {
    name = "echo-remote";
    path = "${./examples/config/${name}/server}";
    partitions = {
      Router = router;
      EchoServer = echo;
    };
  };
  image-echo-direct-xng = xngImage rec {
    name = "echo-direct";
    partitions = {
      EchoClient = echo;
      EchoServer = echo;
    };
  };
  image-echo-local-xng = xngImage {
    name = "echo-local";
    partitions = {
      EchoClient = echo;
      EchoServer = echo;
      Router = router;
    };
  };
  image-echo-remote-client-alt-xng = xngImage rec {
    name = "echo-remote";
    path = "${./examples/config/${name}/client-alt}";
    partitions = {
      EchoClient = echo;
      EchoServer = echo;
      Router = router;
    };
  };
  xng-images = (pkgs.linkFarmFromDrvs "xng-images" [
    image-echo-direct-xng
    image-echo-local-xng
    image-echo-remote-client-xng
    image-echo-remote-server-xng
    image-echo-remote-client-alt-xng
  ]);
}
