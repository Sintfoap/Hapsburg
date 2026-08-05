{
  description = "Hapsburg: a compiled language where everything is mandatory multiple inheritance";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };

        # ferdinand shells out to `cc` at *runtime* to turn its generated C
        # into a native binary, so any packaged output needs a real
        # compiler on PATH, not just at build time. stdenv.cc is the
        # nixpkgs-wrapped compiler (knows where libc/headers live), which
        # is what you want here rather than raw pkgs.gcc.
        cc = pkgs.stdenv.cc;

        hapsburg = pkgs.rustPlatform.buildRustPackage {
          pname = "hapsburg";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;

          nativeBuildInputs = [ pkgs.makeWrapper ];

          # Builds the whole workspace (ferdinand + hapsburg-lsp) and
          # installs both binaries; wrap them so `cc` is always reachable
          # regardless of the caller's shell.
          postFixup = ''
            for bin in $out/bin/*; do
              wrapProgram "$bin" --prefix PATH : ${cc}/bin
            done
          '';

          meta = {
            description = "The Hapsburg language: ferdinand (compiler) and hapsburg-lsp (language server)";
            mainProgram = "ferdinand";
          };
        };
      in
      {
        packages.default = hapsburg;
        packages.hapsburg = hapsburg;

        apps.default = flake-utils.lib.mkApp {
          drv = hapsburg;
          name = "ferdinand";
        };
        apps.ferdinand = flake-utils.lib.mkApp {
          drv = hapsburg;
          name = "ferdinand";
        };
        apps.hapsburg-lsp = flake-utils.lib.mkApp {
          drv = hapsburg;
          name = "hapsburg-lsp";
        };

        devShells.default = pkgs.mkShell {
          packages = [
            pkgs.cargo
            pkgs.rustc
            pkgs.rustfmt
            pkgs.clippy
            pkgs.rust-analyzer
            cc
          ];

          # So rust-analyzer / editor tooling can find std sources.
          RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
        };

        formatter = pkgs.nixpkgs-fmt;
      });
}
