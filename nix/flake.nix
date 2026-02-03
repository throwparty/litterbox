{
  description = "throwparty/litterbox";
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/release-25.11";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    throwparty = {
      url = "git+ssh://git@github.com/throwparty/nix";
      inputs.flake-utils.follows = "flake-utils";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs =
    { flake-utils, nixpkgs, rust-overlay, throwparty, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            (import rust-overlay)
          ];
        };
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          targets = [ "wasm32-wasip1" ];
        };
        inherit (pkgs.lib) getExe getExe';
      in
      {
        devShells.default =
          let
            inherit (throwparty.devShells.${system}) commonTools;
            inherit (pkgs)
              cargo-auditable
              openssl
              pkg-config
              python3
              stdenv
              ;
            toolVersions = throwparty.lib.mkToolVersions {
              inherit pkgs;
              name = "default";
              commands = ''
                ${getExe' rustToolchain "cargo"} --version
                ${getExe cargo-auditable} --version
                ${getExe python3} --version
                ${getExe' rustToolchain "rustc"} --version
              '';
            };
          in
          pkgs.mkShell {
            nativeBuildInputs = [
              cargo-auditable
              openssl
              pkg-config
              python3
              rustToolchain
              stdenv.cc
            ];
            shellHook = "\n cat ${toolVersions}";
          };
      }
    );
}
