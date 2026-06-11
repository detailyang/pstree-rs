{
  description = "pstree-rs — a pstree implementation in Rust for macOS";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachSystem [ "aarch64-darwin" "x86_64-darwin" ] (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "pstree-rs";
          version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package.version;
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;

          buildInputs = [ pkgs.libiconv ];

          # Expose NIX_LIBICONV_PATH for build.rs so it can emit the correct
          # rustc-link-search directive.
          NIX_LIBICONV_PATH = pkgs.libiconv;

          meta = {
            description = "pstree for macOS, written in Rust";
            homepage = "https://github.com/yourusername/pstree-rs";
            license = pkgs.lib.licenses.mit;
            mainProgram = "pstree-rs";
            platforms = pkgs.lib.platforms.darwin;
          };
        };

        apps.default = flake-utils.lib.mkApp {
          drv = self.packages.${system}.default;
        };

        devShells.default = pkgs.mkShell {
          buildInputs = [
            pkgs.rustc
            pkgs.cargo
            pkgs.libiconv
          ];
          NIX_LIBICONV_PATH = pkgs.libiconv;
          LIBRARY_PATH = "${pkgs.libiconv}/lib";
        };
      });
}
