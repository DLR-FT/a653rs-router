{ pkgs, xng-utils }:
xng-utils.lib.buildXngOps {
  inherit pkgs;
  src = pkgs.requireFile {
    name = "14-033.094.ops+armv7a-vmsa-tz+zynq7000.r16736.tbz2";
    url = "http://fentiss.com";
    sha256 = "1gb0cq3mmmr2fqj49p4svx07h5ccs8v564awlsc56mfjhm6jg3n4";
  };
}
