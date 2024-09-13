{
  src,
  craneLib,
  lib,
  stdenv,
  libiconv,
  dockerTools,
  darwin,
  advisory-db,
}:
let
  darwinDeps = [
    libiconv
    darwin.apple_sdk.frameworks.Security
    darwin.apple_sdk.frameworks.SystemConfiguration
  ];

  filteredSrc = lib.cleanSourceWith {
    inherit src;
    filter =
      path: type:
      (lib.hasSuffix ".pem" path) # Load self signed certs to test
      || (lib.hasSuffix ".gql" path) # graphql query
      || (lib.hasSuffix "schema.json" path) # graphql schema
      || (lib.hasSuffix ".snap" path) # insta snapshots
      || (lib.hasSuffix ".json" path) # graphql fixtures
      || (lib.hasSuffix ".kvsd" path) # kvsd fixtures
      || (lib.hasSuffix ".xml" path) # rss fixtures
      || (lib.hasSuffix "categories.toml" path)
      ||
        # Default filter from crane (allow .rs files)
        (craneLib.filterCargoSources path type);
  };

  commonArgs = {
    src = filteredSrc;
    strictDeps = true;
    # Cargo.toml workspace.metadata.crane.version does not work
    version = "1";
    buildInputs = [ ] ++ lib.optionals stdenv.isDarwin darwinDeps;
    CARGO_PROFILE = "dev";
  };

  cargoArtifacts = craneLib.buildDepsOnly commonArgs;

  individualCrateArgs = commonArgs // {
    doCheck = false;
    CARGO_PROFILE = "release";
  };

  dockerImageLabels = {
    "org.opencontainers.image.source" = "https://github.com/ymgyt/syndicationd";
  };

  syndTerm = craneLib.buildPackage (
    individualCrateArgs
    // (
      let
        crate = craneLib.crateNameFromCargoToml { cargoToml = ../../crates/synd_term/Cargo.toml; };
      in
      {
        inherit (crate) pname version;
        cargoExtraArgs = "--package ${crate.pname}";
      }
    )
  );
  syndTermImage = dockerTools.buildImage {
    name = "synd-term";
    tag = "latest";
    config = {
      Cmd = [ "${syndTerm}/bin/synd" ];
      Labels = dockerImageLabels;
    };
  };

  syndApi = craneLib.buildPackage (
    individualCrateArgs
    // (
      let
        crate = craneLib.crateNameFromCargoToml { cargoToml = ../../crates/synd_api/Cargo.toml; };
      in
      {
        inherit (crate) pname version;
        cargoExtraArgs = "--package ${crate.pname}";
      }
    )
  );
  syndApiImage = dockerTools.buildImage {
    name = "synd-api";
    tag = "latest";
    config = {
      Cmd = [ "${syndApi}/bin/synd-api" ];
      Labels = dockerImageLabels;
    };
  };
in
{
  checks = {
    clippy = craneLib.cargoClippy (
      commonArgs
      // {
        inherit cargoArtifacts;
        cargoExtraArgs = "--features integration --exclude synd-perf";
        cargoClippyExtraArgs = "--workspace -- --deny warnings";
      }
    );

    nextest = craneLib.cargoNextest (
      commonArgs
      // {
        inherit cargoArtifacts;
        cargoNextestExtraArgs = "--features integration";
        CARGO_PROFILE = "";
        RUST_LOG = "synd,integration=debug";
        RUST_BACKTRACE = "1";
      }
    );

    audit = craneLib.cargoAudit {
      inherit advisory-db;
      src = filteredSrc;
      cargoAuditExtraArgs =
        let
          ignoreAdvisories = lib.concatStrings (
            lib.strings.intersperse " " (
              map (x: "--ignore ${x}")
                (builtins.fromTOML (builtins.readFile ../../.cargo/audit.toml)).advisories.ignore
            )
          );
        in
        "${ignoreAdvisories}";
    };

    fmt = craneLib.cargoFmt commonArgs;
  };

  packages = {
    synd-term = syndTerm;
    synd-term-image = syndTermImage;
    synd-api = syndApi;
    synd-api-image = syndApiImage;
    coverage = craneLib.cargoLlvmCov (
      commonArgs
      // {
        inherit cargoArtifacts;
        cargoLlvmCovExtraArgs = "--lcov --all-features --output-path $out  --ignore-filename-regex '(client/generated/.*.rs)'";
      }
    );
  };
  inherit darwinDeps;
}
