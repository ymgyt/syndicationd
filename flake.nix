{
  description = "syndicationd";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
    nixpkgs-unstable.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
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

  outputs =
    {
      self,
      nixpkgs,
      nixpkgs-unstable,
      crane,
      rust-overlay,
      flake-utils,
      advisory-db,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;

          # terraform has an unfree license (‘bsl11’)
          config.allowUnfree = true;
        };
        pkgs-unstable = import nixpkgs-unstable { inherit system overlays; };

        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        src = pkgs.lib.cleanSourceWith {
          src = ./.; # The original, unfiltered source
          filter =
            path: type:
            (pkgs.lib.hasSuffix ".pem" path) # Load self signed certs to test
            || (pkgs.lib.hasSuffix ".gql" path) # graphql query
            || (pkgs.lib.hasSuffix "schema.json" path) # graphql schema
            || (pkgs.lib.hasSuffix ".snap" path) # insta snapshots
            || (pkgs.lib.hasSuffix ".json" path) # graphql fixtures
            || (pkgs.lib.hasSuffix ".kvsd" path) # kvsd fixtures
            || (pkgs.lib.hasSuffix ".xml" path) # rss fixtures
            || (pkgs.lib.hasSuffix "categories.toml" path)
            ||
              # Default filter from crane (allow .rs files)
              (craneLib.filterCargoSources path type);
        };

        synd = pkgs.callPackage ./etc/nix/crane.nix { inherit src craneLib advisory-db; };

        ci_packages = with pkgs; [
          # >= 1.31.0 for modules
          pkgs-unstable.just
          nushell # just set nu as shell
          cargo-bundle-licenses
          docker
          terraform
        ];

        # Inherits from checks cargo-nextest, cargo-audit
        dev_packages =
          with pkgs;
          [
            pkgs-unstable.cargo-dist
            typos
            graphql-client
            opentelemetry-collector-contrib
            git-cliff
            cargo-release
            cargo-machete
            cargo-insta
            cargo-flamegraph
            oranda
            gnuplot # for rendering with criterion
            graphviz
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
            exec nu
          '';
        };

        devShells.ci = craneLib.devShell { packages = ci_packages; };

        devShells.ebpf = pkgs.mkShell {
          packages = [ (pkgs.rust-bin.fromRustupToolchainFile ./crates/ebpf/synd_ebpf/rust-toolchain.toml) ];
        };

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
