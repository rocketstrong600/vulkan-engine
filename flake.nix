{
  inputs = {
    naersk.url = "github:nix-community/naersk/master";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, utils, naersk }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        naersk-lib = pkgs.callPackage naersk { };
        shader-slang = pkgs.callPackage ./NixDep/shader-slang.nix { };
      in
      {

        defaultPackage = naersk-lib.buildPackage ./.;
        devShell = with pkgs; mkShell {
          buildInputs = [ cargo rustc rustfmt pre-commit rustPackages.clippy rust-analyzer vulkan-loader vulkan-validation-layers vulkan-tools-lunarg libxkbcommon wayland shader-slang];
          packages = [ vulkan-tools ];
          RUST_SRC_PATH = rustPlatform.rustLibSrc;
          shellHook = ''
            export LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath [ vulkan-loader libxkbcommon wayland]}:$LD_LIBRARY_PATH
            export VK_LAYER_PATH=${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d:$VK_LAYER_PATH
          '';
        };
      }
    );
}
