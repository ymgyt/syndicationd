{
  description = "syndicationd";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    crane = {
      url = "github:ipetkov/crane/v0.17.1";
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
          sha256 = "sha256-opUgs6ckUQCyDxcB9Wy51pqhd0MPGHUVbwRKKPGiwZU=";
        };

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        src = pkgs.lib.cleanSourceWith {
          src = ./.; # The original, unfiltered source
          filter = path: type:
            # Load self signed certs to test
            (pkgs.lib.hasSuffix ".pem" path)
            # insta snapshots
            || (pkgs.lib.hasSuffix ".snap" path)
            || (pkgs.lib.hasSuffix ".xml" path)
            || (pkgs.lib.hasSuffix "categories.toml" path) ||
            # Default filter from crane (allow .rs files)
            (craneLib.filterCargoSources path type);
        };

        darwinDeps = [
          pkgs.libiconv
          pkgs.darwin.apple_sdk.frameworks.Security
          pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
        ];

        commonArgs = {
          inherit src;
          strictDeps = true;

          # pname and version required, so set dummpy values
          pname = "syndicationd-workspace";
          version = "0.1";

          buildInputs = [ ]
            ++ pkgs.lib.optionals pkgs.stdenv.isDarwin darwinDeps;
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        individualCrateArgs = commonArgs // {
          inherit cargoArtifacts;
          # NB: we disable tests since we will run them all via cargo-nextest
          doCheck = false;
        };

        syndTerm = craneLib.buildPackage (individualCrateArgs // (let
          crate = craneLib.crateNameFromCargoToml {
            cargoToml = ./crates/synd_term/Cargo.toml;
          };
        in {
          inherit (crate) pname version;
          cargoExtraArgs = "--package ${crate.pname}";
        }));

        syndApi = craneLib.buildPackage (individualCrateArgs // (let
          crate = craneLib.crateNameFromCargoToml {
            cargoToml = ./crates/synd_api/Cargo.toml;
          };
        in {
          inherit (crate) pname version;
          cargoExtraArgs = "--package ${crate.pname}";
        }));

        checks = {
          inherit syndTerm syndApi;

          clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoExtraArgs = "--features integration";
            cargoClippyExtraArgs = "--workspace -- --deny warnings";
          });

          nextest = craneLib.cargoNextest (commonArgs // {
            inherit cargoArtifacts;
            cargoNextestExtraArgs = "--features integration --no-capture";
            CARGO_PROFILE = "";
            RUST_LOG = "synd,integration=debug";
          });

          audit = craneLib.cargoAudit {
            inherit src advisory-db;
            cargoAuditExtraArgs = let
              ignoreAdvisories = pkgs.lib.concatStrings
                (pkgs.lib.strings.intersperse " " (map (x: "--ignore ${x}")
                  (builtins.fromTOML (builtins.readFile
                    ./.cargo/audit.toml)).advisories.ignore));
            in "--ignore yanked ${ignoreAdvisories}";
          };

          fmt = craneLib.cargoFmt commonArgs;
        };

        syndApiImage = pkgs.dockerTools.buildImage {
          name = "synd-api";
          tag = "latest";
          config = { Cmd = [ "${syndApi}/bin/synd-api" ]; };
        };

        ci_packages = with pkgs; [
          just
          nushell # just set nu as shell
          typos
          cargo-bundle-licenses
          docker
        ];

        # Inherits from checks cargo-nextest, cargo-audit
        dev_packages = with pkgs;
          [
            graphql-client
            opentelemetry-collector-contrib
            git-cliff
            cargo-release
            cargo-machete
            cargo-insta
            # cargo-llvm-cov-0.6.9 is marked as broken,
            # cargo-llvm-cov
            # We need latest cargo-dist which is not available in nixpkgs-unstable now
            # cargo-dist
            oranda
          ] ++ ci_packages
          ## For cargo-release build
          ++ pkgs.lib.optionals pkgs.stdenv.isDarwin darwinDeps;

      in {
        inherit checks;

        packages = {
          default = self.packages."${system}".synd-term;
          synd-term = syndTerm;
          synd-api = syndApi;
        } // pkgs.lib.optionalAttrs (!pkgs.stdenv.isDarwin) {
          coverage = craneLib.cargoLlvmCov (commonArgs // {
            inherit cargoArtifacts;
            # not supported yet in crane
            # cargoLlvmCovCommand = "nextest";
            cargoLlvmCovExtraArgs =
              "--codecov --all-features --output-path $out";
          });

          synd-api-image = syndApiImage;
        };

        apps.default = flake-utils.lib.mkApp {
          drv = syndTerm;
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
