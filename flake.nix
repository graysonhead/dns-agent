{
  description = "Agent to dynamically update DNS based on interface IPs and external IP discovery";
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    pre-commit-hooks.url = "github:cachix/pre-commit-hooks.nix";
  };


  outputs = { self, flake-utils, naersk, nixpkgs, pre-commit-hooks }:

    flake-utils.lib.eachDefaultSystem
      (system:
        let
          pkgs = (import nixpkgs) {
            inherit system;
          };

          naersk' = pkgs.callPackage naersk { };

          buildInputs = with pkgs; [
            openssl.dev
          ];

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];
        in
        rec {
          checks = {
            pre-commit-check = pre-commit-hooks.lib.${system}.run {
              src = ./.;
              hooks = {
                clippy = {
                  enable = true;
                  packageOverrides = {
                    cargo = pkgs.cargo;
                    clippy = pkgs.clippy;
                  };
                };
                rustfmt = {
                  enable = true;
                  package = pkgs.rustfmt;
                };
                nixpkgs-fmt.enable = true;
              };
            };
          };
          defaultPackage = packages.dns-agent;
          nixosModules = rec {
            dns-agent = import ./module.nix;
            default = dns-agent;
          };
          packages =
            {
              dns-agent = naersk'.buildPackage {
                src = ./.;
                nativeBuildInputs = nativeBuildInputs;
                buildInputs = buildInputs;
              };
            };


          devShells.default = pkgs.mkShell {
            inherit (self.checks.${system}.pre-commit-check) shellHook;
            nativeBuildInputs = with pkgs;
              [
                nixpkgs-fmt
                cmake
                rustc
                rustfmt
                cargo
                clippy
              ] ++ buildInputs ++ nativeBuildInputs;
          };
        }
      );
}
