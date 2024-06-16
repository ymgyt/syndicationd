{ pkgs }:
let
  # pkgs.runCommand does not pass src to typos
  typo = pkgs.stdenvNoCC.mkDerivation {
    name = "typo";
    src = ../.;
    doCheck = true;
    nativeBuildInputs = with pkgs; [ typos ];
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
in typo

