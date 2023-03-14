set dotenv-load := true

# list available recipes
_default:
	@just --list

# run the benchmarks
bench filter='':
	cd {{justfile_directory()}}/ww-benchmarks && cargo bench {{filter}}

# cargo check the whole project
check:
	cargo check --workspace

# build the docs and optionally open them
doc-build open='':
	RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps --document-private-items --workspace --release --target-dir target {{open}}

# run the virtual tree with info level logs
run-virtual log_level='info':
	cd {{justfile_directory()}}/ww-virtual-tree && cargo build --release
	RUST_LOG=none,ww_virtual_tree={{log_level}} {{justfile_directory()}}/target/release/ww-virtual-tree

# run the tests in debug and release mode
test:
	cargo insta test --unreferenced warn --all-features --workspace
	cargo insta test --unreferenced warn --all-features --workspace --release
