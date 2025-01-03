{
  self,
  lib,
  ...
}: {
  perSystem = {
    config,
    self',
    inputs',
    pkgs,
    ...
  }: let
    deps = import ./dependencies.nix {inherit pkgs;};
  in {
    devShells.default = pkgs.mkShell {
      packages = deps.all;
    };
  };
}
