{
  description = "ghlint - GitHub SDLC linter as a gh extension";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        packages.default = pkgs.stdenv.mkDerivation {
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

        devShells.default = pkgs.mkShell {
          packages = [
            pkgs.zig_0_15
            pkgs.gh
            pkgs.jq
          ];

          shellHook = ''
            echo "ghlint dev environment"
            echo "  zig version: $(zig version)"
            echo "  gh  version: $(gh --version | head -1)"
          '';
        };
      }
    );
}
