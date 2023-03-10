# list available recipes
_default:
	@just --list

# run the benchmarks
bench filter='':
	cargo bench --features bench {{filter}}

# cargo check the whole project
check:
	cargo check
	cargo check --features virtual-tree

# build the docs and optionally open them
doc-build open='':
	RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps --document-private-items --workspace --release --target-dir target {{open}}

# run the virtual tree with info level logs
run-virtual log_level='info':
	RUST_LOG=none,winter_wonderlights={{log_level}} cargo run --release --features virtual-tree

# run the tests in debug and release mode
test:
	cargo insta test --unreferenced warn --all-features
	cargo insta test --unreferenced warn --all-features --release
