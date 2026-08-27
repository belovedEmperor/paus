{
  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-26.05";

  outputs = {
    self,
    nixpkgs,
  }: let
    system = "x86_64-linux";
    pkgs = nixpkgs.legacyPackages.${system};
  in {
    packages.${system}.default = pkgs.rustPlatform.buildRustPackage {
      pname = "paus";
      version = "0.4.0";
      src = self;
      cargoLock.lockFile = ./Cargo.lock;
    };

    homeManagerModules.default = {
      self,
      lib,
      pkgs,
      config,
      ...
    }: let
      pausPkg = self.packages.${pkgs.system}.default;
    in {
      options.services.paus = {
        enable = lib.mkEnableOption "the paus stopwatch daemon";

        breakRatio = lib.mkOption {
          type = lib.types.enum [
            "Equal"
            "Lazy"
            "Standard"
            "Industrious"
            "Hard"
            "Grinding"
          ];
          default = "Standard";
          description = "Break earned per focus";
        };

        dataDir = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "State dir";
        };
      };

      config = lib.mkIf config.services.paus.enable {
        home.packages = [pausPkg];

        home.file.".config/paus/config.json".text = builtins.toJSON (
          {break_ratio = config.services.paus.breakRatio;}
          // lib.optionalAttrs (config.services.paus.dataDir != null) {data_dir = config.services.paus.dataDir;}
        );

        systemd.user.services.paus = {
          Unit.Description = "paus stopwatch daemon";
          Install.WantedBy = ["default.target"];
          Service = {
            ExecStart = "${pausPkg}/bin/paus daemon run";
            Restart = "on-failure";
          };
        };
      };
    };
  };
}
