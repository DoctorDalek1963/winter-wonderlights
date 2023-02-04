# list available recipes
_default:
	@just --list

# run the benchmarks
bench filter='':
	cargo bench --features bench {{filter}}

# cargo check the whole project
check:
	cargo check --all-features

run-virtual:
	RUST_LOG=none,winter_wonderlights=info cargo run --release --features virtual-tree

# run the tests in debug and release mode
test:
	cargo test --all-features
	cargo test --all-features --release
