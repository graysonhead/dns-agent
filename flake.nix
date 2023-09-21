{
  description = "Agent to dynamically update DNS based on interface IPs and external IP discovery";
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };


  outputs = { self, flake-utils, naersk, nixpkgs }:

    flake-utils.lib.eachDefaultSystem (system:
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
