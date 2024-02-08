{ pkgs, a653rs-linux-hypervisor, partitions, runTest, rustToolchain }:
{
  nixpkgs-fmt = pkgs.runCommand "check-format-nix"
    {
      nativeBuildInputs = [ pkgs.nixpkgs-fmt ];
    } "nixpkgs-fmt --check ${./.} && touch $out";

  cargo-fmt = pkgs.runCommand "check-format-rust"
    {
      nativeBuildInputs = [ rustToolchain ];
    } "cd ${./.} && cargo fmt --check && touch $out";

  integration = import ./examples/config/echo-remote {
    inherit pkgs a653rs-linux-hypervisor partitions runTest;
  };
}
