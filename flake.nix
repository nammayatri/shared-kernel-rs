{
  description = "shared-kernel-rs: shared Rust libraries for the nammayatri ecosystem";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    mission-control.url = "github:Platonic-Systems/mission-control";
    flake-root.url = "github:srid/flake-root";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    namma-dsl-rs = {
      url = "github:nammayatri/namma-dsl-rs";
    };
  };

  outputs = inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];

      imports = [
        inputs.mission-control.flakeModule
        inputs.flake-root.flakeModule
        ./nix/scripts.nix
      ];

      perSystem = { config, system, lib, ... }:
        let
          pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [ inputs.rust-overlay.overlays.default ];
          };

          # Single source of truth: read rust-toolchain.toml at the repo root.
          rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        in
        {
          # TODO: if any crate in this workspace becomes a binary or a
          # downstream flake needs to consume one as a Nix input, expose it
          # here as `packages.default = pkgs.rustPlatform.buildRustPackage { ... }`
          # (or via `crane` for finer-grained dep caching).

          devShells.default = pkgs.mkShell {
            name = "shared-kernel-rs-shell";

            inputsFrom = [
              config.flake-root.devShell
            ];

            packages = [
              rustToolchain
              pkgs.jq
              pkgs.curl
              config.mission-control.wrapper
            ];
          };
        };
    };
}
