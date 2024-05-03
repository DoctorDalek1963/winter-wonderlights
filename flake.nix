{
  description = "A program to render 3D effects on a Christmas tree in real time";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";

    pre-commit-hooks = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs @ {flake-parts, ...}:
    flake-parts.lib.mkFlake {inherit inputs;} {
      imports = [
        inputs.pre-commit-hooks.flakeModule
      ];

      systems = ["x86_64-linux" "aarch64-linux"];
      perSystem = {
        config,
        system,
        ...
      }: let
        pkgs = import inputs.nixpkgs {
          inherit system;
          overlays = [(import inputs.rust-overlay)];
        };

        wasm-bindgen-cli = pkgs.wasm-bindgen-cli.override {
          version = "0.2.92";
          hash = "sha256-1VwY8vQy7soKEgbki4LD+v259751kKxSxmo/gqE6yV0=";
          cargoHash = "sha256-aACJ+lYNEU8FFBs158G1/JG8sc6Rq080PeKCMnwdpH0=";
        };

        buildRustToolchain = pkgs.rust-bin.selectLatestNightlyWith;

        rustToolchain = buildRustToolchain (toolchain: toolchain.default);

        craneLib = (inputs.crane.mkLib pkgs).overrideToolchain rustToolchain;
        src = pkgs.lib.cleanSourceWith {
          src = ./.;
          filter = path: type:
            (pkgs.lib.hasSuffix "\.html" path)
            || (craneLib.filterCargoSources path type);
        };

        commonArgsNativeBuildInputs = with pkgs; [
          # We have to use llvm/clang 15 because there's an issue with clang 16
          # and above that was only fixed in bindgen v0.62.0. We can't use the
          # updated version of bindgen because nokhwa v0.11.0 hasn't been
          # released yet. Once nokhwa v0.11.0 is released, we should be able to
          # remove these clang version overrides
          (rustPlatform.bindgenHook.override {inherit (llvmPackages_15) clang;})

          pkg-config
          git # rs_ws281x has to update a submodule in its build.rs
        ];

        commonArgsBuildInputs = with pkgs; [
          # See above
          llvmPackages_15.libclang.lib

          linuxHeaders # <linux/videodev2.h> for v4l2 for nokhwa
        ];

        graphicalBuildInputs = with pkgs; [
          libGL
          libxkbcommon
          xorg.libX11
          xorg.libxcb
          xorg.libXcursor
          xorg.libXi
          xorg.libXrandr
          wayland
        ];

        env = rec {
          # Server
          DATA_DIR = "/home/dyson/repos/winter-wonderlights/data";
          COORDS_FILENAME = "2020-matt-parker.gift";

          SERVER_SSL_CERT_PATH = "/dev/null";
          SERVER_SSL_KEY_PATH = "/dev/null";

          PORT = "23120";
          LIGHTS_NUM = "250";

          # Client
          SERVER_URL = "ws://localhost:${PORT}";

          # Scanner server
          SCANNER_PORT = "23121";

          # Scanner clients
          SCANNER_SERVER_URL = "ws://localhost:${SCANNER_PORT}";
        };

        commonArgs =
          {
            inherit src;
            strictDeps = true;
            doCheck = false;

            # We set these here because we need to compile system library stuff
            # for cargoArtifacts, which gets built before any of the packages
            nativeBuildInputs = commonArgsNativeBuildInputs;
            buildInputs = commonArgsBuildInputs;
          }
          // env;

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        individualCrateArgs =
          commonArgs
          // {
            inherit cargoArtifacts;
            inherit (craneLib.crateNameFromCargoToml {inherit src;}) version;
          };
      in rec {
        devShells.default = pkgs.mkShell {
          nativeBuildInputs =
            [
              (buildRustToolchain (toolchain:
                toolchain.default.override {
                  extensions = ["rust-analyzer" "rust-src" "rust-std"];
                }))
              pkgs.cargo-nextest
              pkgs.just
            ]
            ++ commonArgsBuildInputs
            ++ commonArgsNativeBuildInputs
            ++ graphicalBuildInputs;
          shellHook = ''
            ${config.pre-commit.installationScript}
          '';
        };

        # See https://flake.parts/options/pre-commit-hooks-nix and
        # https://github.com/cachix/git-hooks.nix/blob/master/modules/hooks.nix
        # for all the available hooks and options
        pre-commit.settings.hooks = {
          check-added-large-files.enable = true;
          check-merge-conflicts.enable = true;
          check-toml.enable = true;
          check-vcs-permalinks.enable = true;
          check-yaml.enable = true;
          end-of-file-fixer.enable = true;
          trim-trailing-whitespace.enable = true;

          rustfmt = {
            enable = true;
            packageOverrides = {
              cargo = rustToolchain;
              rustfmt = rustToolchain;
            };
          };
        };

        checks = {
          inherit (packages) doc;

          bench = craneLib.mkCargoDerivation (commonArgs
            // {
              inherit cargoArtifacts;
              pnameSuffix = "-bench";
              buildPhaseCargoCommand = "cargo bench --features bench";
            });

          clippy = craneLib.cargoClippy (commonArgs
            // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = "--all-targets --features bench -- --deny warnings";
            });

          fmt = craneLib.cargoFmt {
            inherit src;
          };

          nextest = craneLib.cargoNextest (commonArgs
            // {
              inherit cargoArtifacts;
              partitions = 1;
              partitionType = "count";
            });
        };

        packages = {
          doc = craneLib.cargoDoc (commonArgs
            // {
              inherit cargoArtifacts;
              cargoDocExtraArgs = pkgs.lib.concatStringsSep " " [
                "--no-deps"
                "--document-private-items"
                "--workspace"
                "--features"
                (pkgs.lib.concatStringsSep "," ["gift-coord-editor/_driver"])
              ];
              RUSTDOCFLAGS = "--deny warnings";
            });
        };
      };
    };
}
