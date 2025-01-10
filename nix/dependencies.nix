{pkgs, ...}: rec {
  toolchain = pkgs.rust-bin.selectLatestNightlyWith (toolchain:
    toolchain.default.override {
      extensions = ["rust-src"];
    });

  nativeBuildInputs = with pkgs;
    [helix]
    ++ pkgs.lib.optionals pkgs.stdenv.isLinux [mold clang]
    ++ pkgs.lib.optionals pkgs.stdenv.isDarwin []
    ++ pkgs.lib.optionals pkgs.stdenv.isDarwin (
      with pkgs.darwin.apple_sdk.frameworks; []
    );

  buildInputs = with pkgs; [
  ];

  all = [toolchain] ++ nativeBuildInputs ++ buildInputs;
}
