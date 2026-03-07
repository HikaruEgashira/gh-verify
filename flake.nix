{
  description = "ghlint - GitHub SDLC linter as a gh extension";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    devenv.url = "github:cachix/devenv";
    systems.url = "github:nix-systems/default";
  };

  nixConfig = {
    extra-trusted-public-keys = "devenv.cachix.org-1:w1cLUi8dv3hnoSPGAuibQv+f9TZLr6cv/Hm9XgU50cw=";
    extra-substituters = "https://devenv.cachix.org";
  };

  outputs = { self, nixpkgs, devenv, systems, ... } @ inputs:
    let
      forEachSystem = nixpkgs.lib.genAttrs (import systems);
    in {
      packages = forEachSystem (system:
        let pkgs = nixpkgs.legacyPackages.${system}; in {
          default = pkgs.stdenv.mkDerivation {
            pname = "gh-lint";
            version = "0.1.0";
            src = ./.;
            nativeBuildInputs = [ pkgs.zig_0_15 ];
            buildPhase = ''
              zig build -Doptimize=ReleaseSafe --global-cache-dir $TMPDIR/zig-cache
            '';
            installPhase = ''
              mkdir -p $out/bin
              cp zig-out/bin/gh-lint $out/bin/
            '';
          };
        }
      );

      devShells = forEachSystem (system:
        let pkgs = nixpkgs.legacyPackages.${system}; in {
          default = devenv.lib.mkShell {
            inherit inputs pkgs;
            modules = [
              ./devenv.nix
              { devenv.root = builtins.toString self.outPath; }
            ];
          };
        }
      );
    };
}
