{ pkgs, cargoLock, rustPlatform }:
let
  mkExample = { rustPlatform, product, example, features, target }: rustPlatform.buildRustPackage {
    inherit cargoLock;
    pname = example;
    version = "0.1.0";
    src = ./.;
    meta.mainProgram = example;
    buildPhase = ''
      cargo build --release --target "${target}" -p ${product} --example=${example} --features=${pkgs.lib.concatStringsSep "," features}
    '';
    doCheck = target != "armv7a-none-eabi";
    checkPhase = ''
      cargo test --target "${target}" -p ${product} --example=${example} --features=${pkgs.lib.concatStringsSep "," features} --frozen
    '';
    installPhase = ''
      mkdir -p "$out"/{bin,lib}
      if [[ "${target}" = "armv7a-none-eabi" ]]
      then
        cp "target/${target}"/release/examples/*.a "$out/lib"
      else
        cp "target/${target}/release/examples/${example}" "$out/bin"
      fi
    '';
  };
in
rec
{
  configurator-linux = rustPlatform.buildRustPackage rec {
    inherit cargoLock;
    pname = "configurator-linux";
    target = "x86_64-unknown-linux-musl";
    version = "0.1.0";
    src = ./.;
    buildPhase = ''
      cargo build --release -p ${pname} --target ${target}
    '';
    checkPhase = ''
      cargo test -p ${pname} --target ${target} --frozen
    '';
    doCheck = false;
    installPhase = ''
      mkdir -p "$out"/bin
      cp target/${target}/release/${pname} "$out/bin"
    '';
    meta.mainProgram = "configurator-linux";
  };

  configurator-xng = rustPlatform.buildRustPackage rec {
    inherit cargoLock;
    pname = "configurator-xng";
    target = "armv7a-none-eabi";
    version = "0.1.0";
    src = ./.;
    buildPhase = ''
      cargo build --release -p ${pname} --target ${target}
    '';
    checkPhase = ''
      cargo test -p ${pname} --target ${target} --frozen
    '';
    doCheck = false;
    installPhase = ''
      mkdir -p "$out"/lib
      cp target/${target}/release/*.a "$out/lib"
    '';
  };

  echo-linux = rustPlatform.buildRustPackage rec {
    inherit cargoLock;
    target = "x86_64-unknown-linux-musl";
    pname = "echo-linux";
    version = "0.1.0";
    src = ./.;
    buildPhase = ''
      cargo build --release --target ${target} -p ${pname}
    '';
    doCheck = false;
    installPhase = ''
      mkdir -p "$out"/bin
      cp target/${target}/release/echo "$out/bin"
    '';
    meta.mainProgram = "echo";
  };

  echo-xng = rustPlatform.buildRustPackage rec {
    inherit cargoLock;
    target = "armv7a-none-eabi";
    pname = "echo-xng";
    version = "0.1.0";
    src = ./.;
    buildPhase = ''
      cargo build --release --target ${target} -p ${pname}
    '';
    doCheck = false;
    installPhase = ''
      mkdir -p "$out"/lib
      cp "target/${target}"/release/*.a "$out/lib"
    '';
  };

  router-echo-client-linux = mkExample {
    inherit rustPlatform;
    example = "router-echo-client-linux";
    product = "router";
    features = [ "linux" ];
    target = "x86_64-unknown-linux-musl";
  };
  router-echo-server-linux = mkExample {
    inherit rustPlatform;
    example = "router-echo-server-linux";
    product = "router";
    features = [ "linux" ];
    target = "x86_64-unknown-linux-musl";
  };
  router-echo-client-xng = mkExample {
    inherit rustPlatform;
    example = "router-echo-client-xng";
    product = "router";
    features = [ "xng" ];
    target = "armv7a-none-eabi";
  };
  router-echo-server-xng = mkExample {
    inherit rustPlatform;
    example = "router-echo-server-xng";
    product = "router";
    features = [ "xng" ];
    target = "armv7a-none-eabi";
  };
  router-echo-local-xng = mkExample {
    inherit rustPlatform;
    example = "router-echo-local-xng";
    product = "router";
    features = [ "xng" ];
    target = "armv7a-none-eabi";
  };
}
