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
    rustPlatform = pkgs.makeRustPlatform {
      cargo = deps.toolchain;
      rustc = deps.toolchain;
    };
  in {
    packages.minira = rustPlatform.buildRustPackage {
      pname = "minira";
      version = "0.1.0";
      nativeBuildInputs = deps.nativeBuildInputs;
      buildInputs = deps.buildInputs;
      src = ./..;
      cargoLock = {
        lockFile = ../Cargo.lock;
      };

      postInstall = ''
        wrapProgram $out/bin/minira \
          --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath [deps.toolchain]} \
          --prefix DYLD_LIBRARY_PATH : ${lib.makeLibraryPath [deps.toolchain]}
      '';
    };
  };
}
