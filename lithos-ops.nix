{ pkgs, xng-utils }:
xng-utils.lib.buildLithOsOps {
  inherit pkgs;
  patches = [ ./patches/lithos-xng-armv7a-vmsa-tz.lds.patch ];
  src = pkgs.requireFile {
    name = "020.080.ops.r7919+xngsmp.tbz2";
    url = "https://fentiss.com";
    sha256 = "1b73d6x3galw3bhj5nac7ifgp15zrsyipn4imwknr24gp1l14sc8";
  };
}
