{pkgs, ...}: rec {
  toolchain = pkgs.rust-bin.selectLatestNightlyWith (toolchain:
    toolchain.default.override {
      extensions = ["rust-src" "rustc-dev"];
    });

  nativeBuildInputs = with pkgs;
    [openssl pkg-config makeWrapper]
    ++ pkgs.lib.optionals pkgs.stdenv.isLinux [mold clang]
    ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [];

  buildInputs = with pkgs; [
    openssl
    pkg-config
  ];

  shell = with pkgs; [toolchain helix] ++ nativeBuildInputs ++ buildInputs;
}
