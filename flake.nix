{
  description = "Junk Drawer — a minimal GTK4 file manager";

  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAll = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      packages = forAll (pkgs: {
        default = pkgs.callPackage ./nix/package.nix { };
      });

      devShells = forAll (pkgs: {
        default = pkgs.mkShell {
          inputsFrom = [ self.packages.${pkgs.stdenv.hostPlatform.system}.default ];
          packages = with pkgs; [
            clippy
            rustfmt
            rust-analyzer
          ];
        };
      });

      # INT-24: `nix flake check` runs the tests (by building the package), clippy and rustfmt.
      checks = forAll (
        pkgs:
        let
          pkg = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
        in
        {
          tests = pkg;
          clippy = pkg.overrideAttrs (old: {
            pname = "junk-drawer-clippy";
            nativeBuildInputs = old.nativeBuildInputs ++ [ pkgs.clippy ];
            buildPhase = "cargo clippy --all-targets --offline -- -D warnings";
            doCheck = false;
            installPhase = "touch $out";
            postInstall = "";
            dontFixup = true;
          });
          fmt = pkgs.runCommand "junk-drawer-fmt" { nativeBuildInputs = [ pkgs.rustfmt ]; } ''
            cd ${./.}
            rustfmt --check --edition 2024 src/main.rs
            touch $out
          '';
        }
      );

      formatter = forAll (pkgs: pkgs.nixfmt);

      overlays.default = final: _: { junk-drawer = final.callPackage ./nix/package.nix { }; };

      nixosModules.default = import ./nix/nixos-module.nix self;
      homeManagerModules.default = import ./nix/hm-module.nix self;
    };
}
