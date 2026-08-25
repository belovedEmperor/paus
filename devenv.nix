{pkgs, ...}: {
  # https://devenv.sh/packages/
  packages = with pkgs; [];

  # https://devenv.sh/languages/
  languages.rust.enable = true;

  # See full reference at https://devenv.sh/reference/options/
}
