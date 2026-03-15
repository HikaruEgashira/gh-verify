#!/usr/bin/env bash
# Creusot formal verification: translate + prove all predicates.
# Runs outside devenv to avoid OCaml threading conflicts (devenv 2.x + Nix).
#
# Usage:
#   ./scripts/verify.sh            # prove all predicates
#   ./scripts/verify.sh <name>     # prove a single predicate
set -euo pipefail

export OPAMSWITCH=creusot
eval $(opam env --set-switch 2>/dev/null) || {
  echo "Error: Creusot opam switch not found."
  echo "Required versions (from creusot-deps.opam):"
  echo "  why3:     git-56a15760"
  echo "  why3find: git-3a98fc32"
  echo ""
  echo "Setup: opam switch create creusot && ./INSTALL (in creusot checkout)"
  exit 1
}

echo "=== Creusot translate ==="
cargo creusot -p gh-verify-verif

PRED="${1:-*}"
echo ""
echo "=== Creusot prove ($PRED) ==="
cargo creusot prove "$PRED" -- -p gh-verify-verif
