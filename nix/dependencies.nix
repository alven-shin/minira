{pkgs, ...}: rec {
  toolchain = pkgs.rust-bin.stable.latest.default.override {
    extensions = ["rust-src"];
  };

  nativeBuildInputs = with pkgs;
    []
    ++ pkgs.lib.optionals pkgs.stdenv.isLinux [mold clang]
    ++ pkgs.lib.optionals pkgs.stdenv.isDarwin []
    ++ pkgs.lib.optionals pkgs.stdenv.isDarwin (
      with pkgs.darwin.apple_sdk.frameworks; []
    );

  buildInputs = with pkgs; [
  ];

  all = [toolchain] ++ nativeBuildInputs ++ buildInputs;
}
