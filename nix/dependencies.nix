{pkgs, ...}: rec {
  toolchain = pkgs.rust-bin.selectLatestNightlyWith (toolchain:
    toolchain.default.override {
      extensions = ["rust-src" "rustc-dev"];
    });

  nativeBuildInputs = with pkgs;
    [helix zlib curl]
    ++ pkgs.lib.optionals pkgs.stdenv.isLinux [mold clang]
    ++ pkgs.lib.optionals pkgs.stdenv.isDarwin []
    ++ pkgs.lib.optionals pkgs.stdenv.isDarwin (
      with pkgs.darwin.apple_sdk.frameworks; []
    );

  buildInputs = with pkgs; [
  ];

  all = [toolchain] ++ nativeBuildInputs ++ buildInputs;
}
