{
  inputs,
  cell,
}: let
  inherit (inputs.nixpkgs) pkgs;
  inherit (inputs.cells.repo.rust) toolchain;
  buildRustCrateForPkgs = pkgs: pkgs.buildRustCrate.override { rustc = toolchain; cargo = toolchain;};
in {
  # sane default for a binary package
  default = (pkgs.callPackage "${inputs.self}/Cargo.nix" { inherit buildRustCrateForPkgs pkgs; inherit (inputs) nixpkgs;}).rootCrate.build;
}
