{
  description = "Scale the position of IME candidate windows on Wayland";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      forAllSystems = nixpkgs.lib.genAttrs [
        "x86_64-linux"
        "aarch64-linux"
      ];
    in
    {
      packages = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
        in
        {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = cargoToml.package.name;
            version = cargoToml.package.version;
            src = self;
            cargoLock.lockFile = ./Cargo.lock;
            meta = {
              inherit (cargoToml.package) description;
              homepage = cargoToml.package.repository;
              license = pkgs.lib.licenses.mit;
              mainProgram = cargoToml.package.name;
              platforms = pkgs.lib.platforms.linux;
            };
          };
        }
      );
    };
}
