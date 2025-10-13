{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    utils.url = "github:numtide/flake-utils";
    playdate-sdk.url = "github:RegularTetragon/playdate-sdk-flake";
  };

  outputs =
    {
      nixpkgs,
      utils,
      playdate-sdk,
      ...
    }:
    utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
      {
        devShell =
          with pkgs;
          mkShell {
            buildInputs = [
              pkg-config
              gcc-arm-embedded-13
              rust-bindgen
              gnumake
            ]
            ++ (lib.optionals stdenv.isLinux [
              playdate-sdk.packages.x86_64-linux.default
            ]);
          }
          // (lib.optionalAttrs stdenv.isLinux {
            PLAYDATE_SDK_PATH = "${playdate-sdk.packages.x86_64-linux.default}";
          });
      }
    );
}
