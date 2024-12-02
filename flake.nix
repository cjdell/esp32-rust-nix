# cargo generate esp-rs/esp-idf-template cargo

{
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:

    let
      pkgs = import nixpkgs {
        inherit system;
      };
      fhs = pkgs.buildFHSUserEnv {
        name = "fhs-shell";
        targetPkgs = pkgs: with pkgs; [
          gcc

          pkg-config
          libclang.lib
          gnumake
          cmake
          ninja
          libz

          git
          wget
          zed-editor
          tio
          nushell

          rustup
          espup
          cargo-generate

          espflash
          python3
          python3Packages.pip
          python3Packages.virtualenv
          ldproxy

          deno
        ];

        runScript = pkgs.writeScript "run.sh" ''
          #! ${pkgs.stdenv.shell}
          #set -euxo pipefail

          espup install
          source ~/export-esp.sh

          rustup install stable

          # Fix rust-analyzer by installing from the stable toolchain and forcing it into the PATH...
          rustup component add --toolchain stable rust-analyzer
          mkdir -p ~/rust-analyzer
          rm -f ~/rust-analyzer/rust-analyzer
          ln -s $(rustup which --toolchain stable rust-analyzer) ~/rust-analyzer/rust-analyzer
          mkdir -p ~/.config/rust-analyzer

          PATH=~/rust-analyzer:$PATH zeditor --foreground .
          # PATH=~/rust-analyzer:$PATH code .

          # ${pkgs.nushell}/bin/nu
        '';
      };
    in
    {
      devShell = fhs.env;
    });
}
