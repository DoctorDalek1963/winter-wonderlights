{
  description = "A program to render 3D effects on a Christmas tree in real time";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.11";
    flake-parts.url = "github:hercules-ci/flake-parts";

    pre-commit-hooks = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
  };

  outputs = inputs @ {
    self,
    flake-parts,
    ...
  }:
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
          overlays = [
            (import inputs.rust-overlay)
            (_final: prev: {
              wasm-bindgen-cli = prev.wasm-bindgen-cli.override {
                version = "0.2.92";
                hash = "sha256-1VwY8vQy7soKEgbki4LD+v259751kKxSxmo/gqE6yV0=";
                cargoHash = "sha256-aACJ+lYNEU8FFBs158G1/JG8sc6Rq080PeKCMnwdpH0=";
              };
            })
          ];
        };

        # Merge two attribute sets deeply by joining lists and recursively
        # merging sets. If a value is neither a list nor a set, the value given
        # in the second set overwrites the one in the first.
        merge = setA: setB: let
          mergeSingle = a: b:
            if (builtins.isList a && builtins.isList b)
            then a ++ b
            else if (builtins.isAttrs a && builtins.isAttrs b)
            then merge a b
            else if (builtins.isString a && builtins.isString b)
            then "${a} ${b}"
            else b;
          aWithB = builtins.listToAttrs (map (x: {
            name = x;
            value =
              if setB ? ${x}
              then (mergeSingle setA.${x} setB.${x})
              else setA.${x};
          }) (builtins.attrNames setA));
          unmergedB = builtins.listToAttrs (map (x: {
            name = x;
            value = setB.${x};
          }) (builtins.filter (x: !(aWithB ? ${x})) (builtins.attrNames setB)));
        in
          aWithB // unmergedB;

        buildRustToolchain = pkgs.rust-bin.selectLatestNightlyWith;

        rustToolchain = buildRustToolchain (toolchain: toolchain.default);

        craneLib = (inputs.crane.mkLib pkgs).overrideToolchain rustToolchain;

        buildSrc = {
          # The crates which should be included in the source. Each element
          # must be a path segment and doesn't have to be a crate name.
          # "drivers" would include everything in the drivers directory, for
          # example.
          crates, # listOf nonEmptyStr
          # Should we include the data directory?
          includeData ? false, # bool
          # List the suffix of any extra filetypes you want to include.
          extraSuffices ? [], # listOf nonEmptyStr
        }: let
          inherit (pkgs) lib;
        in
          pkgs.lib.cleanSourceWith {
            src = self;
            filter = orig_path: type: let
              path = toString orig_path;
              base = baseNameOf path;
              parentDir = baseNameOf (dirOf path);

              matchesSuffix = lib.any (suffix: lib.hasSuffix suffix base) extraSuffices;
              isInCrate = lib.any (crateName: lib.hasInfix crateName path) crates;
              dataCheck = includeData && lib.hasInfix parentDir "data/";
            in
              (type == "directory")
              || isInCrate
              || dataCheck
              || matchesSuffix
              || (base == "Cargo.toml")
              || (base == "Cargo.lock")
              # The workspace references every crate and cargo needs to know
              # they exist and are properly configured, so we need a main.rs or
              # lib.rs even for crates that we don't use
              || (parentDir == "src" && (base == "main.rs" || base == "lib.rs"))
              # The ww-benchmarks crate needs to see its benching code
              || (parentDir == "benches" && lib.hasSuffix ".rs" base)
              || (parentDir == ".cargo" && base == "config.toml");
          };

        fullSrc = pkgs.lib.cleanSourceWith {
          src = ./.;
          filter = path: type:
            (pkgs.lib.hasSuffix "\.html" path)
            || (pkgs.lib.hasSuffix "\.txt" path)
            || (pkgs.lib.hasSuffix "\.snap" path)
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
          mesa
          vulkan-loader
          vulkan-validation-layers
          xorg.libX11
          xorg.libxcb
          xorg.libXcursor
          xorg.libXi
          xorg.libXrandr
          wayland
        ];

        gzipInputs = with pkgs; [gnutar gzip];

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

        localDevEnv = env // {DATA_DIR = "/home/dyson/repos/winter-wonderlights/data";};

        commonArgs = src:
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

        cargoArtifacts = src: craneLib.buildDepsOnly (commonArgs src);

        individualCrateArgs = src:
          (commonArgs src)
          // {
            cargoArtifacts = cargoArtifacts fullSrc;
            inherit (craneLib.crateNameFromCargoToml {inherit src;}) version;
          };
      in rec {
        devShells.default = pkgs.mkShell (rec {
            nativeBuildInputs =
              [
                (buildRustToolchain (toolchain:
                  toolchain.default.override {
                    extensions = ["rust-analyzer" "rust-src" "rust-std"];
                    targets = ["wasm32-unknown-unknown"];
                  }))
              ]
              ++ (with pkgs; [
                cargo-deny
                cargo-insta
                cargo-nextest
                cargo-watch
                trunk
                wasm-bindgen-cli
                just
              ])
              ++ commonArgsBuildInputs
              ++ commonArgsNativeBuildInputs
              ++ graphicalBuildInputs;
            shellHook = ''
              ${config.pre-commit.installationScript}
              export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath nativeBuildInputs}"
            '';
            # This eliminates a warning in the virtual tree about vulkan validation layers
            VK_LAYER_PATH = "${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d";
          }
          // localDevEnv);

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

        checks =
          packages # Make sure all the packages build successfully
          // {
            clippy = craneLib.cargoClippy ((commonArgs fullSrc)
              // {
                cargoArtifacts = cargoArtifacts fullSrc;
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
              src = fullSrc;
            };

            nextest = craneLib.cargoNextest ((commonArgs fullSrc)
              // {
                cargoArtifacts = cargoArtifacts fullSrc;
                partitions = 1;
                partitionType = "count";
              });

            insta-test = craneLib.mkCargoDerivation ((commonArgs fullSrc)
              // {
                cargoArtifacts = cargoArtifacts fullSrc;
                pnameSuffix = "-insta";
                buildPhaseCargoCommand = pkgs.lib.concatStringsSep "\n" (map (args @ {crate, ...}: let
                  flags =
                    if args ? "features"
                    then "--no-default-features --features ${args.features}"
                    else "--all-features";
                in
                  # bash
                  ''
                    cd ${crate}
                    cargo insta test --unreferenced reject ${flags}
                    cd ..
                  '') [
                  # We only need to run insta on crates with snapshot tests
                  {crate = "ww-effects";}
                  {crate = "ww-frame";}
                ]);
                nativeBuildInputs = (commonArgs fullSrc).nativeBuildInputs ++ [pkgs.cargo-insta];
              });

            deny-with-virtual-tree =
              craneLib.cargoDeny ((commonArgs fullSrc)
                // {cargoDenyExtraArgs = ''--features "ww-server/driver-virtual-tree"'';});

            deny-with-raspi-ws2811 =
              craneLib.cargoDeny ((commonArgs fullSrc)
                // {cargoDenyExtraArgs = ''--features "ww-server/driver-raspi-ws2811 ww-scanner-server/driver-raspi-ws2811 gift-coord-editor/driver-raspi-ws2811"'';});
          };

        packages = let
          rustToolchainWasm = buildRustToolchain (toolchain:
            toolchain.default.override {
              targets = ["wasm32-unknown-unknown"];
            });

          craneLibTrunk =
            ((inputs.crane.mkLib pkgs).overrideToolchain rustToolchainWasm)
            .overrideScope (_: _: {inherit (pkgs) wasm-bindgen-cli;});

          benchPkg = args:
            craneLib.mkCargoDerivation ((commonArgs fullSrc)
              // {
                cargoArtifacts = cargoArtifacts fullSrc;
                pnameSuffix = "-bench";
                buildPhaseCargoCommand = "cd ww-benchmarks && cargo bench ${args} | tee output.txt && cd ..";
                postInstall = "cp ww-benchmarks/output.txt $out/output.txt";
              });

          # Make a package with overridable environment variables
          mkEnvPkg = binaryName: src: crateArgs: extraWrapArgs:
            pkgs.lib.makeOverridable (overridableEnv @ {
              DATA_DIR,
              COORDS_FILENAME,
              SERVER_SSL_CERT_PATH,
              SERVER_SSL_KEY_PATH,
              PORT,
              LIGHTS_NUM,
              SERVER_URL,
              SCANNER_PORT,
              SCANNER_SERVER_URL,
            }:
              craneLib.buildPackage (merge ((individualCrateArgs src)
                  // overridableEnv # Also inject the new env vars into the build
                  // {
                    nativeBuildInputs = commonArgsNativeBuildInputs ++ [pkgs.makeWrapper];
                    postInstall = let
                      wrapProgramArgs = pkgs.lib.concatStringsSep " " ([
                          ''--set DATA_DIR "${DATA_DIR}"''
                          ''--set COORDS_FILENAME "${COORDS_FILENAME}"''
                          ''--set SERVER_SSL_CERT_PATH "${SERVER_SSL_CERT_PATH}"''
                          ''--set SERVER_SSL_KEY_PATH "${SERVER_SSL_KEY_PATH}"''
                          ''--set PORT "${PORT}"''
                          ''--set LIGHTS_NUM "${LIGHTS_NUM}"''
                          ''--set SERVER_URL "${SERVER_URL}"''
                          ''--set SCANNER_PORT "${SCANNER_PORT}"''
                          ''--set SCANNER_SERVER_URL "${SCANNER_SERVER_URL}"''
                        ]
                        ++ extraWrapArgs);
                    in ''
                      wrapProgram "$out/bin/${binaryName}" ${wrapProgramArgs}
                    '';
                    meta.mainProgram = binaryName;
                  })
                # Merge with crateArgs, extending lists where applicable. This
                # allows us to easily add extra things to buildInputs, for
                # example
                crateArgs))
            localDevEnv;
        in {
          bench = benchPkg "";
          bench-ci = benchPkg "-- --output-format bencher";

          doc = craneLib.cargoDoc ((commonArgs fullSrc)
            // {
              cargoArtifacts = cargoArtifacts fullSrc;
              cargoDocExtraArgs = pkgs.lib.concatStringsSep " " [
                "--no-deps"
                "--document-private-items"
                "--workspace"
                "--features"
                (pkgs.lib.concatStringsSep "," ["gift-coord-editor/_driver"])
              ];
              RUSTDOCFLAGS = "--deny warnings";
            });

          doc-with-deps = craneLib.cargoDoc ((commonArgs fullSrc)
            // {
              cargoArtifacts = cargoArtifacts fullSrc;
              cargoDocExtraArgs = pkgs.lib.concatStringsSep " " [
                "--document-private-items"
                "--workspace"
                "--features"
                (pkgs.lib.concatStringsSep "," ["gift-coord-editor/_driver"])
              ];
              RUSTDOCFLAGS = "--deny warnings";
            });

          server-debug =
            mkEnvPkg "ww-server" (buildSrc {
              includeData = true;
              crates = [
                "drivers/debug"
                "ww-driver-trait"
                "ww-effects"
                "ww-frame"
                "ww-server"
                "ww-shared"
                "ww-shared-server-tls"
              ];
            }) {
              pname = "ww-server-debug";
              cargoExtraArgs = "--package=ww-server --no-default-features --features driver-debug";
              buildInputs = gzipInputs;
            } [
              ''--prefix PATH : "${pkgs.lib.makeBinPath gzipInputs}"''
            ];

          server-raspi-ws2811 =
            mkEnvPkg "ww-server" (buildSrc {
              includeData = true;
              crates = [
                "drivers/raspi-ws2811"
                "ww-driver-trait"
                "ww-effects"
                "ww-frame"
                "ww-server"
                "ww-shared"
                "ww-shared-server-tls"
              ];
            }) {
              pname = "ww-server-raspi-ws2811";
              cargoExtraArgs = "--package=ww-server --no-default-features --features driver-raspi-ws2811";
              buildInputs = gzipInputs;
            } [
              ''--prefix PATH : "${pkgs.lib.makeBinPath gzipInputs}"''
            ];

          # Overriding this one is a little complicated but the virtual tree
          # should only be used for development, so a local DATA_DIR isn't a
          # big issue
          server-virtual-tree =
            pkgs.lib.makeOverridable
            (virtual-tree-runner:
              mkEnvPkg "ww-server" (buildSrc {
                includeData = true;
                crates = [
                  "drivers/virtual-tree"
                  "ww-driver-trait"
                  "ww-effects"
                  "ww-frame"
                  "ww-server"
                  "ww-shared"
                  "ww-shared-server-tls"
                ];
              }) {
                pname = "ww-server-virtual-tree";
                cargoExtraArgs = "--package=ww-server --no-default-features --features driver-virtual-tree";
                buildInputs = gzipInputs;
              } [
                ''--prefix PATH : "${pkgs.lib.makeBinPath gzipInputs}"''
                ''--set CARGO_BIN_FILE_VIRTUAL_TREE_RUNNER "${virtual-tree-runner}/bin/virtual-tree-runner"''
              ])
            (mkEnvPkg "virtual-tree-runner" (buildSrc {
                crates = [
                  "virtual-tree"
                  "ww-effects"
                  "ww-frame"
                  "ww-gift-coords"
                ];
              }) {
                pname = "virtual-tree-runner";
                cargoExtraArgs = "--package=virtual-tree-runner";
                buildInputs = graphicalBuildInputs;
              } [
                ''--prefix LD_LIBRARY_PATH : "${pkgs.lib.makeLibraryPath graphicalBuildInputs}"''
              ]);

          client-native =
            mkEnvPkg "ww-client" (buildSrc {
              crates = ["ww-client" "ww-effects" "ww-shared"];
            }) {
              pname = "ww-client-native";
              cargoExtraArgs = "--package=ww-client";
            } [
              ''--prefix LD_LIBRARY_PATH : "${pkgs.lib.makeLibraryPath graphicalBuildInputs}"''
            ];

          client-web = pkgs.lib.makeOverridable (overridableEnv:
            craneLibTrunk.buildTrunkPackage (
              (individualCrateArgs (buildSrc {
                crates = ["ww-client" "ww-effects" "ww-shared"];
                extraSuffices = [".html"];
              }))
              // overridableEnv # Also inject the new env vars into the build
              // {
                pname = "ww-client-web";
                cargoExtraArgs = "--package=ww-client";

                trunkIndexPath = "ww-client/index.html";
                CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
                inherit (pkgs) wasm-bindgen-cli;
              }
            ))
          env;

          gift-coord-editor-raspi-ws2811 =
            mkEnvPkg "gift-coord-editor" (buildSrc {
              crates = [
                "drivers/raspi-ws2811"
                "gift-coord-editor"
                "ww-driver-trait"
                "ww-frame"
                "ww-gift-coords"
              ];
            }) {
              pname = "gift-coord-editor";
              cargoExtraArgs = "--package=gift-coord-editor --features driver-raspi-ws2811";
            } [];

          scanner-server-raspi-ws2811 =
            mkEnvPkg "ww-scanner-server" (buildSrc {
              includeData = true;
              crates = [
                "drivers/raspi-ws2811"
                "scanner/server"
                "scanner/shared"
                "ww-driver-trait"
                "ww-frame"
                "ww-gift-coords"
                "ww-shared-server-tls"
              ];
            }) {
              pname = "ww-server-raspi-ws2811";
              cargoExtraArgs = "--package=ww-scanner-server --no-default-features --features driver-raspi-ws2811";
            } [];

          scanner-client-native =
            mkEnvPkg "ww-scanner-client" (buildSrc {
              crates = ["scanner/client" "scanner/shared"];
            }) {
              pname = "ww-scanner-client-native";
              cargoExtraArgs = "--package=ww-scanner-client";
            } [
              ''--prefix LD_LIBRARY_PATH : "${pkgs.lib.makeLibraryPath graphicalBuildInputs}"''
            ];

          scanner-client-web = pkgs.lib.makeOverridable (overridableEnv:
            craneLibTrunk.buildTrunkPackage (
              (individualCrateArgs (buildSrc {
                crates = ["scanner/client" "scanner/shared"];
                extraSuffices = [".html"];
              }))
              // overridableEnv # Also inject the new env vars into the build
              // {
                pname = "ww-scanner-client-web";
                cargoExtraArgs = "--package=ww-scanner-client";

                trunkIndexPath = "scanner/client/index.html";
                CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
                inherit (pkgs) wasm-bindgen-cli;
              }
            ))
          env;
        };
      };
    };
}
