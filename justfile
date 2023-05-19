set dotenv-load := true

# list available recipes
_default:
	@just --list

# run the benchmarks
bench filter='':
	cd {{justfile_directory()}}/ww-benchmarks && cargo bench {{filter}}

# check the crates with optional flags
_check flags='':
	cd {{justfile_directory()}}/ww-server && cargo check {{flags}}
	cd {{justfile_directory()}}/ww-virtual-tree && cargo check {{flags}}
	cd {{justfile_directory()}}/ww-client && cargo check {{flags}}
	cd {{justfile_directory()}}/ww-client && cargo check --target wasm32-unknown-unknown {{flags}}

# cargo check the whole project
check:
	@just _check
	@just _check --release

# build the docs and optionally open them
doc-build open='':
	RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps --document-private-items --workspace --release --target-dir target {{open}}

# run the virtual tree with info level logs
run-virtual log_level='info':
	cd {{justfile_directory()}}/ww-virtual-tree && cargo build --release
	RUST_LOG=none,ww_virtual_tree={{log_level}} {{justfile_directory()}}/target/release/ww-virtual-tree

# a little convenience function to build the server and client
_build_server_client flags='':
	cd {{justfile_directory()}}/ww-server && cargo build {{flags}}
	cd {{justfile_directory()}}/ww-client && trunk build {{flags}}

# build the server and client in debug mode
build: _build_server_client

# build the server and client in release mode
build-release: (_build_server_client '--release')

# watch the server and rerun anytime the code is changed
watch-server flags='':
	cd {{justfile_directory()}}/ww-server && cargo watch -x "run {{flags}}"

# serve the client with Trunk
serve-client flags='':
	cd {{justfile_directory()}}/ww-client && trunk serve {{flags}}

# run the tests in debug and release mode
test:
	cd {{justfile_directory()}}/ww-effects && cargo insta test --unreferenced reject --all-features
	cd {{justfile_directory()}}/ww-effects && cargo insta test --unreferenced reject --all-features --release
	cd {{justfile_directory()}}/ww-gift-coords && cargo insta test --unreferenced reject --all-features
	cd {{justfile_directory()}}/ww-gift-coords && cargo insta test --unreferenced reject --all-features --release
