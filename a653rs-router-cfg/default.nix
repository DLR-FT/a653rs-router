{ rustPlatform, cargoLock, ... }:
rustPlatform.buildRustPackage rec {
  inherit cargoLock;

  pname = "a653rs-router-cfg";
  version = "0.1.0";
  src = ./..;
  doCheck = true;
  meta.mainProgram = pname;
}
