{
  description = "syndicationd";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/release-23.11";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, fenix, crane, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ fenix.overlays.default ];
        pkgs = import nixpkgs { inherit system overlays; };
        rustToolchain = fenix.packages.${system}.fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = "sha256-e4mlaJehWBymYxJGgnbuCObVlqMlQSilZ8FljG9zPHY=";
        };

        craneLib = crane.lib.${system}.overrideToolchain rustToolchain;

        src = pkgs.lib.cleanSourceWith {
          src = ./.; # The original, unfiltered source
          filter = path: type:
            # Load self signed certs to test
            (pkgs.lib.hasSuffix ".pem" path) ||
            # Default filter from crane (allow .rs files)
            (craneLib.filterCargoSources path type);
        };

        commonArgs = {
          inherit src;
          strictDeps = true;

          # pname and version required, so set dummpy values
          pname = "syndicationd-workspace";
          version = "0.1";

          buildInputs = [ ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.libiconv
            pkgs.darwin.apple_sdk.frameworks.Security
            pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
          ];
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        syndTermCrate = craneLib.crateNameFromCargoToml {
          cargoToml = ./crates/synd_term/Cargo.toml;
        };
        syndTerm = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          inherit (syndTermCrate) pname version;
          cargoExtraArgs = "--package ${syndTermCrate.pname}";
        });

        syndApiCrate = craneLib.crateNameFromCargoToml {
          cargoToml = ./crates/synd_api/Cargo.toml;
        };
        syndApi = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          inherit (syndApiCrate) pname version;
          cargoExtraArgs = "--package ${syndApiCrate.pname}";
        });

        checks = {
          inherit syndTerm syndApi;

          clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoExtraArgs = "--features integration";
            cargoClippyExtraArgs = "--workspace -- --deny warnings";
          });

          nextest = craneLib.cargoNextest (commonArgs // {
            inherit cargoArtifacts;
            cargoExtraArgs = "--features integration --no-capture";
            CARGO_PROFILE = "";
            RUST_LOG = "synd,integration=debug";
          });

          fmt = craneLib.cargoFmt commonArgs;
        };

        ci_packages = with pkgs; [
          just
          nushell # just set nu as shell
        ];

        dev_packages = with pkgs;
          [
            cargo-nextest
            graphql-client
            nixfmt
            # Failed to run proc-macro server from path /nix/store/z1vlkv6nccjd523iwp5p6pdkr2abm9jq-rust-1.76.0/libexec/rust-analyzer-proc-macro-srv,
            # rust-analyzer
            opentelemetry-collector-contrib
            git-cliff
          ] ++ ci_packages ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [ ];

      in {
        inherit checks;

        packages.default = self.packages."${system}".synd;
        packages.synd = syndTerm;
        packages.synd_api = syndApi;

        apps.default = flake-utils.lib.mkApp { drv = syndTerm; };

        devShells.default = craneLib.devShell {
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
