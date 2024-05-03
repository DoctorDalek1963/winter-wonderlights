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
            || (pkgs.lib.hasSuffix "\.txt" path)
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
          DATA_DIR = ./data;
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
          inherit (packages) bench doc;

          clippy = craneLib.cargoClippy (commonArgs
            // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = pkgs.lib.concatStringsSep " " [
                "--no-deps"
                "--"
                "-D absolute-paths-not-starting-with-crate"
                "-D dead-code"
                "-D missing-abi"
                "-D missing-docs"
                "-D redundant-semicolons"
                "-D unsafe-op-in-unsafe-fn"
                "-D unused-attributes"
                "-D unused-import-braces"
                "-D unused-lifetimes"
                "-W noop-method-call"
                "-W single-use-lifetimes"
                "-W trivial-numeric-casts"
                "-W unused-macro-rules"
                "-W variant-size-differences"
                "-W clippy::cargo"
                "-W clippy::complexity"
                "-D clippy::correctness"
                "-A clippy::nursery"
                "-A clippy::pedantic"
                "-W clippy::perf"
                "-A clippy::restriction"
                "-W clippy::style"
                "-W clippy::suspicious"
                "-A clippy::cargo-common-metadata"
                "-A clippy::cognitive-complexity"
                "-A clippy::derivable-impls"
                "-A clippy::multiple-crate-versions"
                "-A clippy::needless-update"
                "-D clippy::allow-attributes-without-reason"
                "-D clippy::dbg-macro"
                "-D clippy::empty-structs-with-brackets"
                "-D clippy::get-unwrap"
                "-D clippy::missing-assert-message"
                "-D clippy::missing-docs-in-private-items"
                "-D clippy::rest-pat-in-fully-bound-structs"
                "-D clippy::self-named-module-files"
                "-D clippy::semicolon-if-nothing-returned"
                "-D clippy::tests-outside-test-module"
                "-D clippy::todo"
                "-D clippy::trait-duplication-in-bounds"
                "-D clippy::type-repetition-in-bounds"
                "-D clippy::undocumented-unsafe-blocks"
                "-D clippy::unicode-not-nfc"
                "-D clippy::uninlined-format-args"
                "-D clippy::unnecessary-self-imports"
                "-D clippy::unseparated-literal-suffix"
                "-D clippy::used-underscore-binding"
                "-W clippy::as-ptr-cast-mut"
                "-W clippy::borrow-as-ptr"
                "-W clippy::branches-sharing-code"
                "-W clippy::checked-conversions"
                "-W clippy::clear-with-drain"
                "-W clippy::cloned-instead-of-copied"
                "-W clippy::collection-is-never-read"
                "-W clippy::debug-assert-with-mut-call"
                "-W clippy::derive-partial-eq-without-eq"
                "-W clippy::doc-markdown"
                "-W clippy::empty-line-after-doc-comments"
                "-W clippy::empty-line-after-outer-attr"
                "-W clippy::equatable-if-let"
                "-W clippy::expect-used"
                "-W clippy::explicit-deref-methods"
                "-W clippy::explicit-into-iter-loop"
                "-W clippy::explicit-iter-loop"
                "-W clippy::fallible-impl-from"
                "-W clippy::filter-map-next"
                "-W clippy::fn-params-excessive-bools"
                "-W clippy::from-iter-instead-of-collect"
                "-W clippy::implicit-clone"
                "-W clippy::inconsistent-struct-constructor"
                "-W clippy::index-refutable-slice"
                "-W clippy::inefficient-to-string"
                "-W clippy::items-after-statements"
                "-W clippy::iter-not-returning-iterator"
                "-W clippy::iter-on-empty-collections"
                "-W clippy::iter-on-single-items"
                "-W clippy::iter-with-drain"
                "-W clippy::large-stack-arrays"
                "-W clippy::large-types-passed-by-value"
                "-W clippy::manual-assert"
                "-W clippy::manual-clamp"
                "-W clippy::manual-instant-elapsed"
                "-W clippy::manual-let-else"
                "-W clippy::manual-ok-or"
                "-W clippy::manual-string-new"
                "-W clippy::many-single-char-names"
                "-W clippy::map-unwrap-or"
                "-W clippy::match-bool"
                "-W clippy::match-on-vec-items"
                "-W clippy::match-same-arms"
                "-W clippy::mismatching-type-param-order"
                "-W clippy::missing-errors-doc"
                "-W clippy::missing-fields-in-debug"
                "-W clippy::missing-panics-doc"
                "-W clippy::mut-mut"
                "-W clippy::needless-bitwise-bool"
                "-W clippy::needless-collect"
                "-W clippy::needless-continue"
                "-W clippy::needless-for-each"
                "-W clippy::needless-pass-by-value"
                "-W clippy::option-option"
                "-W clippy::or-fun-call"
                "-W clippy::path-buf-push-overwrite"
                "-W clippy::ptr-as-ptr"
                "-W clippy::ptr-cast-constness"
                "-W clippy::range-minus-one"
                "-W clippy::range-plus-one"
                "-W clippy::rc-buffer"
                "-W clippy::rc-mutex"
                "-W clippy::redundant-clone"
                "-W clippy::ref-option-ref"
                "-W clippy::same-functions-in-if-condition"
                "-W clippy::same-name-method"
                "-W clippy::significant-drop-in-scrutinee"
                "-W clippy::single-char-lifetime-names"
                "-W clippy::single-match-else"
                "-W clippy::stable-sort-primitive"
                "-W clippy::string-add-assign"
                "-W clippy::suboptimal-flops"
                "-W clippy::suspicious-operation-groupings"
                "-W clippy::trailing-empty-array"
                "-W clippy::trivially-copy-pass-by-ref"
                "-W clippy::trivial-regex"
                "-W clippy::unimplemented"
                "-W clippy::unnecessary-box-returns"
                "-W clippy::unnecessary-join"
                "-W clippy::unnecessary-safety-comment"
                "-W clippy::unnecessary-safety-doc"
                "-W clippy::unnecessary-wraps"
                "-W clippy::unneeded-field-pattern"
                "-W clippy::unnested-or-patterns"
                "-W clippy::unreadable-literal"
                "-W clippy::unused-peekable"
                "-W clippy::unused-rounding"
                "-W clippy::unused-self"
                "-W clippy::unwrap-in-result"
                "-W clippy::unwrap-used"
                "-W clippy::useless-let-if-seq"
                "-W clippy::use-self"
                "-W clippy::zero-sized-map-values"
              ];
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
          bench = craneLib.mkCargoDerivation (commonArgs
            // {
              inherit cargoArtifacts;
              pnameSuffix = "-bench";
              buildPhaseCargoCommand = "cargo bench --features bench";
            });

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
