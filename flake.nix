{
  description = "Wayland backend + shell tooling for Slint";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      crane,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        craneLib = crane.mkLib pkgs;

        src =
          let
            filterSlintFiles = path: _type: builtins.match ".*slint$" path != null;
            cargoOrSlint = path: type: (filterSlintFiles path type) || (craneLib.filterCargoSources path type);
          in
          pkgs.lib.cleanSourceWith {
            src = ./.;
            name = "source";
            filter = cargoOrSlint;
          };

        runtimeLibs = with pkgs; [
          libGL
          libxkbcommon
          wayland
          fontconfig
          vulkan-loader
          stdenv.cc.cc.lib
        ];

        commonArgs = {
          inherit src;
          strictDeps = true;
          nativeBuildInputs = with pkgs; [
            pkg-config
            makeWrapper
          ];
          buildInputs = runtimeLibs;
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        shell = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;
            # Wrap all output binaries
            postFixup = ''
              for bin in "$out"/bin/*; do
                [ -f "$bin" ] || continue
                wrapProgram "$bin" \
                  --prefix LD_LIBRARY_PATH : ${pkgs.lib.makeLibraryPath runtimeLibs}
              done
            '';
          }
        );
      in
      {
        packages.default = shell;
        devShells.default = craneLib.devShell {
          packages = with pkgs; [
            slint-lsp
            pkg-config
          ];
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath runtimeLibs;
          PKG_CONFIG_PATH = pkgs.lib.makeSearchPathOutput "dev" "lib/pkgconfig" (
            runtimeLibs
            ++ [
              pkgs.freetype
            ]
          );
        };
      }
    );
}
