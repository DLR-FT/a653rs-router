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

  a653rs-router-linux = rustPlatform.buildRustPackage rec {
    inherit cargoLock;
    pname = "a653rs-router-linux";
    target = "x86_64-unknown-linux-musl";
    version = "0.1.0";
    src = ./.;
    buildPhase = ''
      cargo build \
        --release \
        --target ${target} \
        --package ${pname} \
        --features partition,log,trace \
        --bin partition
    '';
    doCheck = true;
    installPhase = ''
      mkdir -p "$out/bin"
      cp "target/${target}"/release/partition "$out/bin/router"
    '';
  };

  a653rs-router-zynq7000 = rustPlatform.buildRustPackage rec {
    inherit cargoLock;
    pname = "a653rs-router-zynq7000";
    target = "armv7a-none-eabi";
    version = "0.1.0";
    src = ./.;
    buildPhase = ''
      cargo build \
        --release \
        --target ${target} \
        --package ${pname} \
        --features partition \
        --example partition
    '';
    doCheck = false;
    installPhase = ''
      mkdir -p "$out/lib"
      cp "target/${target}"/release/examples/libpartition.a "$out/lib/librouter.a"
    '';
  };
}
