# Nix Requirements

flake.nix with: nixpkgs, rust-overlay (oxalica), naersk, flake-utils
Outputs: packages.default (binary), devShells.default (rust-analyzer + cargo tools)
direnv integration: use flake
Cross-platform: x86_64-linux, x86_64-darwin, aarch64-darwin
Reproducibility: flake.lock pins all inputs; bit-for-bit reproducible builds
