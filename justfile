set dotenv-load

export DATA_DIR := justfile_directory() + "/data"

# list available recipes
_default:
	@just --list

# run the benchmarks
bench filter='':
	cd {{justfile_directory()}}/ww-benchmarks && cargo bench {{filter}}

# check the crates with optional flags
_check flags='':
	cd {{justfile_directory()}}/ww-server && cargo check --no-default-features --features driver-debug {{flags}}
	cd {{justfile_directory()}}/ww-server && cargo check --no-default-features --features driver-virtual-tree {{flags}}
	cd {{justfile_directory()}}/ww-client && cargo check {{flags}}
	cd {{justfile_directory()}}/ww-client && cargo check --target wasm32-unknown-unknown {{flags}}

# cargo check the whole project
check:
	@just _check
	@just _check --release

# build the docs and optionally open them
doc-build open='':
	RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items --workspace --release --target-dir target {{open}}

# a little convenience function to build the server and client
_build_server_client driver flags='':
	cd {{justfile_directory()}}/ww-server && cargo build --no-default-features --features {{driver}} {{flags}}
	cd {{justfile_directory()}}/ww-client && trunk build {{flags}}

# build the server and client in debug mode
build driver:
	@just _build_server_client {{driver}}

# build the server and client in release mode
build-release driver:
	@just _build_server_client {{driver}} '--release'

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
