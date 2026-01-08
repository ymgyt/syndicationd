{
  description = "syndicationd";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    crane.url = "github:ipetkov/crane/v0.22.0";

    flake-utils.url = "github:numtide/flake-utils";

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      crane,
      rust-overlay,
      flake-utils,
      advisory-db,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        config = {
          # terraform has an unfree license (‘bsl11’)
          allowUnfreePredicate = pkg: builtins.elem (pkg.pname) [ "terraform" ];
        };
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays config; };

        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        synd = pkgs.callPackage ./etc/nix/crane.nix {
          inherit craneLib advisory-db;
          src = ./.;
        };

        ci_packages = with pkgs; [
          # >= 1.31.0 for modules
          just
          # ~> 1.9.0 for remote workspace
          terraform
          nushell # just set nu as shell
          cargo-bundle-licenses
          docker
        ];

        # Inherits from checks cargo-nextest, cargo-audit
        dev_packages =
          with pkgs;
          [
            cargo-dist
            cargo-insta
            sqlx-cli
            typos
            graphql-client
            opentelemetry-collector-contrib
            git-cliff
            cargo-release
            cargo-machete
            cargo-flamegraph
            oranda
            gnuplot # for rendering with criterion
            graphviz
            sqlite
          ]
          ++ ci_packages
          ## For cargo-release build
          ++ pkgs.lib.optionals pkgs.stdenv.isDarwin synd.darwinDeps;
      in
      {
        checks = {
          inherit (synd.checks)
            clippy
            nextest
            audit
            fmt
            ;
          typo = pkgs.callPackage ./etc/nix/typo.nix { };
        };

        packages =
          {
            default = self.packages."${system}".synd-term;
            inherit (synd.packages) synd-term synd-api;
          }
          // pkgs.lib.optionalAttrs (!pkgs.stdenv.isDarwin) {
            inherit (synd.packages) coverage synd-term-image synd-api-image;
          };

        apps.default = flake-utils.lib.mkApp {
          drv = synd.packages.synd-term;
          name = "synd";
        };

        devShells.default = craneLib.devShell {
          # Inherit inputs from checks
          checks = self.checks.${system};
          packages = dev_packages;
          shellHook = ''
            # export DATABASE_URL="sqlite://$(pwd)/crates/synd_api/synd.db"
            export SQLX_OFFLINE=true

            exec nu
          '';
        };

        devShells.ci = craneLib.devShell { packages = ci_packages; };

        formatter = pkgs.nixfmt-rfc-style;
      }
    );

  nixConfig = {
    extra-substituters = [ "https://syndicationd.cachix.org" ];
    extra-trusted-public-keys = [
      "syndicationd.cachix.org-1:qeH9C3xDqR831xEuDcfhGEUslMMjGroPPMOwiZfyiJQ="
    ];
  };
}
