# list available recipes
_default:
	@just --list

# run the benchmarks
bench filter='':
	cargo bench --features bench {{filter}}

# cargo check the whole project
check:
	cargo check --all-features

# run the tests in debug and release mode
test:
	cargo test --all-features
	cargo test --all-features --release
