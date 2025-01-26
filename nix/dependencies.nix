{pkgs, ...}: rec {
  toolchain = pkgs.rust-bin.selectLatestNightlyWith (toolchain:
    toolchain.default.override {
      extensions = ["rust-src" "rustc-dev"];
    });

  nativeBuildInputs = with pkgs;
    [openssl pkg-config]
    ++ pkgs.lib.optionals pkgs.stdenv.isLinux [mold clang]
    ++ pkgs.lib.optionals pkgs.stdenv.isDarwin []
    ++ pkgs.lib.optionals pkgs.stdenv.isDarwin (
      with pkgs.darwin.apple_sdk.frameworks; []
    );

  buildInputs = with pkgs; [
    openssl
    pkg-config
  ];

  shell = with pkgs; [toolchain helix] ++ nativeBuildInputs ++ buildInputs;
}
