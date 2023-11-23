{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem
      (system:
        let
          pkgs = import nixpkgs {
            inherit system;
          };
          nativeBuildInputs = with pkgs;[ pkg-config lld tracy ];
          buildInputs = with pkgs; [
              udev alsa-lib vulkan-loader
              xorg.libX11 xorg.libXcursor xorg.libXi xorg.libXrandr # To use the x11 feature
              libxkbcommon wayland # To use the wayland feature
            ];
        in
        with pkgs;
        {
          devShells.default = mkShell {
            inherit buildInputs nativeBuildInputs;
            LD_LIBRARY_PATH = lib.makeLibraryPath buildInputs;
          };
        }
      );
}

# { pkgs ? import <nixpkgs> { } }:
# with pkgs;
# mkShell rec {
#   nativeBuildInputs = [
#     pkg-config rustup
#   ];
#   buildInputs = [

#   ];
#   RUSTC_VERSION =
#     builtins.elemAt
#       (builtins.match
#         ".*channel *= *\"([^\"]*)\".*"
#         (pkgs.lib.readFile ./rust-toolchain.toml)
#       )
#       0;
#   LD_LIBRARY_PATH = lib.makeLibraryPath buildInputs;
#   shellHook = ''
#     rustup toolchain install ''${RUSTC_VERSION}
#   '';
# }
