{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-20.09";
  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.naersk = {
    url = "github:nmattia/naersk";
    inputs.nixpkgs.follows = "nixpkgs";
  };
  inputs.rust-overlay = {
    url = "github:oxalica/rust-overlay";
    inputs.nixpkgs.follows = "nixpkgs";
  };
  inputs.flake-compat = {
    url = "github:edolstra/flake-compat";
    flake = false;
  };

  outputs = { nixpkgs, flake-utils, rust-overlay, naersk, self, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlay naersk.overlay ];
        };
        rust = pkgs.rust-bin.beta.latest.default;
        inherit (pkgs.rust-bin.nightly.latest) cargo;
        naersk-lib = pkgs.naersk.override {
          inherit cargo;
          rustc = rust;
        };
        rust-dev = rust.override {
          extensions = [ "rust-src" "clippy" ];
        };
      in rec {
        packages = {
          hauntbot = naersk-lib.buildPackage rec {
            name = "hauntbot";
            version = "unstable";
            root = ./.;
            nativeBuildInputs = with pkgs; [ llvmPackages.llvm pkgconfig ];
            buildInputs = with pkgs; [ stdenv.cc.libc openssl ];
            override = x: (x // {
              LIBCLANG_PATH = "${pkgs.llvmPackages.libclang}/lib";
              preConfigure = ''
              export BINDGEN_EXTRA_CLANG_ARGS="-isystem ${pkgs.clang}/resource-root/include $NIX_CFLAGS_COMPILE"
              '';
            });
            overrideMain = x: (x // rec {
              name = "${pname}-${version}";
              pname = "hauntbot";
              version =
                let
                  rev = self.shortRev or null;
                in
                  if rev != null then "unstable-${rev}" else "dirty";
            });
          };
        };

        defaultPackage = packages.hauntbot;

        devShell = pkgs.mkShell {
          inputsFrom = packages.hauntbot.builtDependencies;
          nativeBuildInputs = with pkgs; [ sqliteInteractive rust-dev ];
        };
      }
    );
}
