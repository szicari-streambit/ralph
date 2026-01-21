# ABOUTME: Nix flake for reproducible Ralph CLI builds
# ABOUTME: Provides packages.default (binary) and devShells.default (development environment)
{
  description = "Ralph CLI - Automated PRD implementation using GitHub Copilot";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, naersk }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        # Rust toolchain
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        # Naersk for building Rust packages
        naersk' = pkgs.callPackage naersk {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };

      in {
        # Binary package
        packages.default = naersk'.buildPackage {
          src = ./.;
          nativeBuildInputs = with pkgs; [ pkg-config makeWrapper ];
          buildInputs = with pkgs; [ openssl ];

          # Ensure agent templates are bundled into the package output
          postInstall = ''
            mkdir -p "$out/share/ralph/templates/.github"
            if [ -d templates/.github/agents ]; then
              cp -R templates/.github/agents "$out/share/ralph/templates/.github/"
            fi

            # Wrap the ralph binary so RALPH_SHARE_DIR is set to $out/share/ralph
            wrapProgram "$out/bin/ralph" --set RALPH_SHARE_DIR "$out/share/ralph"
          '';
        };

        # Development shell
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            # Rust toolchain
            rustToolchain

            # Build tools
            pkg-config
            openssl

            # Development tools
            cargo-watch
            cargo-edit
            cargo-outdated

            # Linting and formatting
            clippy

            # Git
            git
          ];

          shellHook = ''
            echo "Ralph development environment"
            echo "Rust: $(rustc --version)"
            echo "Cargo: $(cargo --version)"
          '';

          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
        };
      }
    );
}

