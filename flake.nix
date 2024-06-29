{
  description = "syndicationd";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    crane = {
      url = "github:ipetkov/crane/v0.17.3";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-utils.url = "github:numtide/flake-utils";

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, fenix, crane, flake-utils, advisory-db, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ fenix.overlays.default ];
        pkgs = import nixpkgs { inherit system overlays; };
        rustToolchain = fenix.packages.${system}.fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = "sha256-Ngiz76YP4HTY75GGdH2P+APE/DEIx2R/Dn+BwwOyzZU=";
        };

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        src = pkgs.lib.cleanSourceWith {
          src = ./.; # The original, unfiltered source
          filter = path: type:
            (pkgs.lib.hasSuffix ".pem" path) # Load self signed certs to test
            || (pkgs.lib.hasSuffix ".gql" path) # graphql query
            || (pkgs.lib.hasSuffix "schema.json" path) # graphql schema
            || (pkgs.lib.hasSuffix ".snap" path) # insta snapshots
            || (pkgs.lib.hasSuffix ".kvsd" path) # kvsd fixtures
            || (pkgs.lib.hasSuffix ".xml" path) # rss fixtures
            || (pkgs.lib.hasSuffix "categories.toml" path) ||
            # Default filter from crane (allow .rs files)
            (craneLib.filterCargoSources path type);
        };

        synd = pkgs.callPackage ./nix/crane.nix {
          inherit src craneLib advisory-db;
        };

        ci_packages = with pkgs; [
          just
          nushell # just set nu as shell
          cargo-bundle-licenses
          docker
        ];

        # Inherits from checks cargo-nextest, cargo-audit
        dev_packages = with pkgs;
          [
            # rust-analyzer-nightly
            typos
            graphql-client
            opentelemetry-collector-contrib
            git-cliff
            cargo-release
            cargo-machete
            cargo-insta
            oranda
          ] ++ ci_packages
          ## For cargo-release build
          ++ pkgs.lib.optionals pkgs.stdenv.isDarwin synd.darwinDeps;

      in {
        checks = {
          inherit (synd.checks) clippy nextest audit fmt;
          typo = pkgs.callPackage ./nix/typo.nix { };
        };

        packages = {
          default = self.packages."${system}".synd-term;
          inherit (synd.packages) synd-term synd-api;

        } // pkgs.lib.optionalAttrs (!pkgs.stdenv.isDarwin) {
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
            exec nu
          '';
        };

        devShells.ci = craneLib.devShell { packages = ci_packages; };
      });

  nixConfig = {
    extra-substituters = [ "https://syndicationd.cachix.org" ];
    extra-trusted-public-keys = [
      "syndicationd.cachix.org-1:qeH9C3xDqR831xEuDcfhGEUslMMjGroPPMOwiZfyiJQ="
    ];
  };
}
