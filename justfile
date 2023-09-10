set dotenv-load

export DATA_DIR := justfile_directory() + "/data"

# list available recipes
_default:
	@just --list

# run the benchmarks
bench filter='':
	cd {{justfile_directory()}}/ww-benchmarks && cargo bench {{filter}}

# build the docs and optionally open them
doc-build open='':
	RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items --workspace --release --target-dir target {{open}}

# a little convenience function to build the server and client
_build_server_client driver flags='':
	cd {{justfile_directory()}}/ww-server && cargo build --no-default-features --features {{driver}} {{flags}}
	cd {{justfile_directory()}}/ww-client && trunk build {{flags}}

# build the server and client in debug mode
build driver flags='':
	@just _build_server_client {{driver}} {{flags}}

# build the server and client in release mode
build-release driver flags='':
	@just _build_server_client {{driver}} '--release' {{flags}}

# watch the server and rerun anytime the code is changed
watch-server flags='':
	cd {{justfile_directory()}}/ww-server && cargo watch -x "run {{flags}}"

# watch the server with the virtual-tree driver and rerun anytime the code is changed
watch-server-virtual-tree flags='':
	cd {{justfile_directory()}}/ww-server && cargo watch -x "run --no-default-features --features driver-virtual-tree {{flags}}"

# serve the client with Trunk
serve-client flags='':
	cd {{justfile_directory()}}/ww-client && trunk serve {{flags}}

# run the tests in debug and release mode
test:
	cd {{justfile_directory()}}/ww-effects && cargo insta test --unreferenced reject --all-features
	cd {{justfile_directory()}}/ww-effects && cargo insta test --unreferenced reject --all-features --release
	cd {{justfile_directory()}}/ww-frame && cargo insta test --unreferenced reject --all-features
	cd {{justfile_directory()}}/ww-frame && cargo insta test --unreferenced reject --all-features --release
	cd {{justfile_directory()}}/ww-gift-coords && cargo insta test --unreferenced reject --all-features
	cd {{justfile_directory()}}/ww-gift-coords && cargo insta test --unreferenced reject --all-features --release
	cd {{justfile_directory()}}/ww-server && cargo insta test --unreferenced reject --no-default-features --features driver-debug
	cd {{justfile_directory()}}/ww-server && cargo insta test --unreferenced reject --no-default-features --features driver-debug --release

# Build things in CI, according to the specified build type
ci-build build-type flags='':
	#!/usr/bin/env bash
	set -euxo pipefail

	case "{{build-type}}" in
		'client')
			rustup target add wasm32-unknown-unknown
			cd {{justfile_directory()}}/ww-client
			trunk build {{flags}}
		;;

		'driver-debug')
			cd {{justfile_directory()}}/ww-server
			cargo build --no-default-features --features driver-debug {{flags}}
		;;

		'driver-virtual-tree')
			cd {{justfile_directory()}}/ww-server
			cargo build --no-default-features --features driver-virtual-tree {{flags}}
		;;

		'driver-raspi-ws2811')
			cd {{justfile_directory()}}/ww-server
			sudo apt-get install libclang-dev
			# TODO: Compile for Raspberry Pi architecture (use cross?)
			cargo build --no-default-features --features driver-raspi-ws2811 {{flags}}
		;;

		*)
			echo "ERROR: Unrecognised build-type"
			exit 1
		;;
	esac

# TODO: Deny clippy::multiple-unsafe-ops-per-block once it works properly

# run clippy over the whole project
clippy args='':
	cargo clippy --no-deps -- \
	-D absolute-paths-not-starting-with-crate \
	-D missing-abi \
	-D missing-docs \
	-D redundant-semicolons \
	-D unsafe-op-in-unsafe-fn \
	-D unused-import-braces \
	-D unused-lifetimes \
	-W noop-method-call \
	-W single-use-lifetimes \
	-W trivial-numeric-casts \
	-W unused-macro-rules \
	-W unused-tuple-struct-fields \
	-W variant-size-differences \
	-W clippy::cargo \
	-W clippy::complexity \
	-D clippy::correctness \
	-A clippy::nursery \
	-A clippy::pedantic \
	-W clippy::perf \
	-A clippy::restriction \
	-W clippy::style \
	-W clippy::suspicious \
	-A clippy::cognitive-complexity \
	-A clippy::derivable-impls \
	-A clippy::needless-update \
	-D clippy::allow-attributes-without-reason \
	-D clippy::dbg-macro \
	-D clippy::empty-structs-with-brackets \
	-D clippy::get-unwrap \
	-D clippy::missing-assert-message \
	-D clippy::missing-docs-in-private-items \
	-D clippy::rest-pat-in-fully-bound-structs \
	-D clippy::self-named-module-files \
	-D clippy::semicolon-if-nothing-returned \
	-D clippy::tests-outside-test-module \
	-D clippy::todo \
	-D clippy::trait-duplication-in-bounds \
	-D clippy::type-repetition-in-bounds \
	-D clippy::undocumented-unsafe-blocks \
	-D clippy::unicode-not-nfc \
	-D clippy::uninlined-format-args \
	-D clippy::unnecessary-self-imports \
	-D clippy::unseparated-literal-suffix \
	-D clippy::used-underscore-binding \
	-W clippy::as-ptr-cast-mut \
	-W clippy::borrow-as-ptr \
	-W clippy::branches-sharing-code \
	-W clippy::checked-conversions \
	-W clippy::clear-with-drain \
	-W clippy::cloned-instead-of-copied \
	-W clippy::collection-is-never-read \
	-W clippy::debug-assert-with-mut-call \
	-W clippy::derive-partial-eq-without-eq \
	-W clippy::doc-markdown \
	-W clippy::empty-line-after-doc-comments \
	-W clippy::empty-line-after-outer-attr \
	-W clippy::equatable-if-let \
	-W clippy::explicit-deref-methods \
	-W clippy::explicit-into-iter-loop \
	-W clippy::explicit-iter-loop \
	-W clippy::fallible-impl-from \
	-W clippy::filter-map-next \
	-W clippy::fn-params-excessive-bools \
	-W clippy::from-iter-instead-of-collect \
	-W clippy::implicit-clone \
	-W clippy::inconsistent-struct-constructor \
	-W clippy::index-refutable-slice \
	-W clippy::inefficient-to-string \
	-W clippy::items-after-statements \
	-W clippy::iter-not-returning-iterator \
	-W clippy::iter-on-empty-collections \
	-W clippy::iter-on-single-items \
	-W clippy::iter-with-drain \
	-W clippy::large-stack-arrays \
	-W clippy::large-types-passed-by-value \
	-W clippy::manual-assert \
	-W clippy::manual-clamp \
	-W clippy::manual-instant-elapsed \
	-W clippy::manual-let-else \
	-W clippy::manual-ok-or \
	-W clippy::manual-string-new \
	-W clippy::many-single-char-names \
	-W clippy::map-unwrap-or \
	-W clippy::match-bool \
	-W clippy::match-on-vec-items \
	-W clippy::match-same-arms \
	-W clippy::mismatching-type-param-order \
	-W clippy::missing-errors-doc \
	-W clippy::missing-fields-in-debug \
	-W clippy::missing-panics-doc \
	-W clippy::mut-mut \
	-W clippy::needless-bitwise-bool \
	-W clippy::needless-collect \
	-W clippy::needless-continue \
	-W clippy::needless-for-each \
	-W clippy::needless-pass-by-value \
	-W clippy::option-option \
	-W clippy::or-fun-call \
	-W clippy::path-buf-push-overwrite \
	-W clippy::ptr-as-ptr \
	-W clippy::ptr-cast-constness \
	-W clippy::range-minus-one \
	-W clippy::range-plus-one \
	-W clippy::rc-buffer \
	-W clippy::rc-mutex \
	-W clippy::redundant-clone \
	-W clippy::ref-option-ref \
	-W clippy::same-functions-in-if-condition \
	-W clippy::same-name-method \
	-W clippy::significant-drop-in-scrutinee \
	-W clippy::single-char-lifetime-names \
	-W clippy::single-match-else \
	-W clippy::stable-sort-primitive \
	-W clippy::string-add-assign \
	-W clippy::suboptimal-flops \
	-W clippy::suspicious-operation-groupings \
	-W clippy::trailing-empty-array \
	-W clippy::trivially-copy-pass-by-ref \
	-W clippy::trivial-regex \
	-W clippy::unimplemented \
	-W clippy::unnecessary-box-returns \
	-W clippy::unnecessary-join \
	-W clippy::unnecessary-safety-comment \
	-W clippy::unnecessary-safety-doc \
	-W clippy::unnecessary-wraps \
	-W clippy::unneeded-field-pattern \
	-W clippy::unnested-or-patterns \
	-W clippy::unreadable-literal \
	-W clippy::unused-peekable \
	-W clippy::unused-rounding \
	-W clippy::unused-self \
	-W clippy::unwrap-in-result \
	-W clippy::unwrap-used \
	-W clippy::useless-let-if-seq \
	-W clippy::use-self \
	-W clippy::zero-sized-map-values \
	{{args}}
