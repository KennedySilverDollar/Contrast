{
  description = "Contrast dev shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, rust-overlay }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ rust-overlay.overlays.default ];
      };
      rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
    in {
      devShells.${system}.default = pkgs.mkShell {
        packages = [
          rust
          pkgs.cargo-mutants
          pkgs.cargo-semver-checks
          pkgs.cargo-deny
        ];
      };
    };
}