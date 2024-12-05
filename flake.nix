{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    esp32 = {
      url = "github:svelterust/esp32";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, esp32 }:
    let
      pkgs = import nixpkgs { system = "x86_64-linux"; };
      idf-rust = esp32.packages.x86_64-linux.esp32;
      buildInputs = [
        idf-rust
        pkgs.libz
        pkgs.ldproxy
        pkgs.cmake
        pkgs.tio
        pkgs.mkspiffs
      ];
    in
    {
      devShells.x86_64-linux.default = pkgs.mkShell {
        inherit buildInputs;

        shellHook = ''
          export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath buildInputs}:$LD_LIBRARY_PATH"
          export LD_LIBRARY_PATH="${pkgs.stdenv.cc.cc.lib.outPath}/lib:$LD_LIBRARY_PATH"

          export PATH="${idf-rust}/.rustup/toolchains/esp/bin:$PATH"
          export RUST_SRC_PATH="$(rustc --print sysroot)/lib/rustlib/src/rust/src"
        '';
      };
    };
}

# # cargo generate esp-rs/esp-idf-template cargo

# {
#   inputs = {
#     nixpkgs.url = "nixpkgs/nixos-unstable";
#     flake-utils.url = "github:numtide/flake-utils";
#   };

#   outputs =
#     { self, nixpkgs, flake-utils }:
#     flake-utils.lib.eachDefaultSystem (system:

#     let
#       pkgs = import nixpkgs {
#         inherit system;
#       };
#       fhs = pkgs.buildFHSUserEnv {
#         name = "fhs-shell";
#         targetPkgs = pkgs: with pkgs; [
#           gcc

#           pkg-config
#           libclang.lib
#           gnumake
#           cmake
#           ninja
#           libz

#           git
#           wget
#           zed-editor
#           tio
#           nushell

#           rustup
#           espup
#           cargo-generate

#           espflash
#           python3
#           python3Packages.pip
#           python3Packages.virtualenv
#           ldproxy
#           mkspiffs

#           deno
#         ];

#         runScript = pkgs.writeScript "run.sh" ''
#           #! ${pkgs.stdenv.shell}
#           #set -euxo pipefail

#           espup install
#           source ~/export-esp.sh

#           rustup install stable

#           # Fix rust-analyzer by installing from the stable toolchain and forcing it into the PATH...
#           rustup component add --toolchain stable rust-analyzer
#           mkdir -p ~/rust-analyzer
#           rm -f ~/rust-analyzer/rust-analyzer
#           ln -s $(rustup which --toolchain stable rust-analyzer) ~/rust-analyzer/rust-analyzer
#           mkdir -p ~/.config/rust-analyzer

#           # PATH=~/rust-analyzer:$PATH zeditor --foreground .
#           # PATH=~/rust-analyzer:$PATH code .

#           export PATH=~/rust-analyzer:$PATH

#           ${pkgs.nushell}/bin/nu
#         '';
#       };
#     in
#     {
#       devShell = fhs.env;
#     });
# }
