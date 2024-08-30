{ stdenvNoCC, typos }:
let
  # pkgs.runCommand does not pass src to typos
  typo = stdenvNoCC.mkDerivation {
    name = "typo";
    src = ../.;
    doCheck = true;
    nativeBuildInputs = [ typos ];
    dontBuild = true;
    checkPhase = ''
      echo "Typo checking..."
      typos --version
      typos
    '';
    installPhase = ''
      mkdir "$out"
    '';
  };
in
typo
