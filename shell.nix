# Fallback for non-flake Nix users: nix-shell
{ pkgs ? import <nixpkgs> {} }:
pkgs.mkShell {
  packages = [
    pkgs.zig_0_15
    pkgs.gh
    pkgs.jq
  ];
}
