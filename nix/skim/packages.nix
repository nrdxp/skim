{
  inputs,
  cell,
}: let
  inherit (inputs.nixpkgs) pkgs;
  inherit (inputs.cells.repo.rust) toolchain;
  buildRustCrateForPkgs = pkgs:
    pkgs.buildRustCrate.override {
      rustc = toolchain;
      cargo = toolchain;
    };
in {
  # sane default for a binary package
  default =
    (pkgs.callPackage "${inputs.self}/Cargo.nix" {
      inherit buildRustCrateForPkgs pkgs;
      inherit (inputs) nixpkgs;
    })
    .rootCrate
    .build
    .overrideAttrs (_: {
      postInstall = ''
        cp bin/sk-tmux $out/bin
        mkdir -p $out/share
        cp -r man $out/share
        cp -r shell $out/share/skim
      '';
    });
}
