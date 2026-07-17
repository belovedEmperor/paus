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
      version = "0.3.0";
      src = self;
      cargoLock.lockFile = ./Cargo.lock;
    };

    homeManagerModules.default = {pkgs, ...}: let
      paus-pkg = self.packages.${pkgs.system}.default;
    in {
      systemd.user.services.paus = {
        Unit.Description = "paus stopwatch daemon";
        Install.WantedBy = ["default.target"]; # Start on login
        Service = {
          ExecStart = "${paus-pkg}/bin/paus daemon run";
          Restart = "on-failure";
        };
      };

      home.packages = [paus-pkg];
    };
  };
}
