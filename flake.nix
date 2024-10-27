# cargo generate esp-rs/esp-idf-template cargo

{
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
  };

  outputs =
    { self
    , nixpkgs
    }:
    let
      pkgs = import nixpkgs {
        system = "x86_64-linux";
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

          rustup
          espup
          cargo-generate

          espflash
          python3
          python3Packages.pip
          python3Packages.virtualenv
          ldproxy
        ];

        runScript = pkgs.writeScript "run.sh" ''
          #! ${pkgs.stdenv.shell}
          espup install
          source ~/export-esp.sh

          # Fix rust-analyzer by installing from the stable toolchain and forcing it into the PATH...
          rustup component add --toolchain stable rust-analyzer
          mkdir -p ~/rust-analyzer
          ln -s $(rustup which --toolchain stable rust-analyzer) ~/rust-analyzer/rust-analyzer
          export PATH=~/rust-analyzer:$PATH

          bash
        '';
      };
    in
    {
      devShells.${pkgs.system}.default = fhs.env;
    };
}

# {
#   inputs = {
#     nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
#     nixpkgs-esp-dev = {
#       url = "github:mirrexagon/nixpkgs-esp-dev";
#       inputs.nixpkgs.follows = "nixpkgs";
#       # inputs.flake-utils.follows = "flake-utils";
#     };
#     esp32 = {
#       url = "github:svelterust/esp32";
#       inputs.nixpkgs.follows = "nixpkgs";
#     };
#   };

#   outputs = {
#     self,
#     nixpkgs,
#     nixpkgs-esp-dev,
#     esp32,
#   }: let
#     system = "x86_64-linux";
#     pkgs = import nixpkgs {
#         inherit system;
#         overlays = [ (import "${nixpkgs-esp-dev}/overlay.nix") ];
#     };
#     idf-rust = esp32.packages.${system}.esp32;
#   in {
#     devShells.${system}.default = pkgs.mkShell {
#       buildInputs = [
#         pkgs.esp-idf-full
#         idf-rust
#         pkgs.tio
#         pkgs.openssl
#       ];

#       nativeBuildInputs = [ pkgs.pkg-config ];

#       LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [ pkgs.openssl ];

#       shellHook = ''
#         export PATH="${idf-rust}/.rustup/toolchains/esp/bin:$PATH"
#         export RUST_SRC_PATH="$(rustc --print sysroot)/lib/rustlib/src/rust/src"
#       '';
#     };
#   };
# }
