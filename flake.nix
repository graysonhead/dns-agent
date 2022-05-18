{
  description = "Agent to dynamically update DNS based on interface IPs and external IP discovery";

  inputs = {
    cargo2nix.url = "github:cargo2nix/cargo2nix/master";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
    rust-overlay.inputs.flake-utils.follows = "flake-utils";
    nixpkgs.url = "github:nixos/nixpkgs";
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, cargo2nix, flake-utils, rust-overlay, ... }:

    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [(import "${cargo2nix}/overlay")
                      rust-overlay.overlay];
        };

        rustPkgs = pkgs.rustBuilder.makePackageSet' {
          rustChannel = "1.60.0";
          packageFun = import ./Cargo.nix;
        };

        workspaceShell = rustPkgs.workspaceShell {};

        ci = pkgs.rustBuilder.runTests rustPkgs.workspace.cargo2nix {
        };

      in rec {
        packages.dns-agent = (rustPkgs.workspace.dns-agent {}).bin;
        defaultPackage = packages.dns-agent;
        devShell = workspaceShell;
      }
    );
}
