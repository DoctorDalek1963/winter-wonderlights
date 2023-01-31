# list available recipes
_default:
	@just --list

# run the benchmarks
bench filter='':
	cargo bench --features bench {{filter}}

# run the tests in debug and release mode
test:
	cargo test
	cargo test --release
