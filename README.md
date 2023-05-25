# Winter WonderLights

This is a Rust project designed to display 3D effects on a Christmas tree using individually
addressable RGB LEDs. It was directly inspired by [this video by Matt
Parker](https://www.youtube.com/watch?v=TvlpIojusBE).

## Quickstart

#### Dependencies

To compile the program yourself, you will need [Rust](https://rustup.rs/) and you will need to
install a few things with `cargo`. I highly recommend using `cargo binstall`, which you can install
with
```bash
cargo install cargo-binstall
```
or with
```bash
curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
```
if you don't want to wait for it to compile.

Then install the necessary dependencies with
```bash
cargo binstall just trunk
```
replacing `binstall` with `install` if you want to compile from scratch.

If you're setting up for development, also install `cargo-insta` and update the `DATA_DIR` variable
in `.env`.

#### Compiling the program

Winter WonderLights uses a system of different drivers to decide how the effects get displayed. See
`ww-server/Cargo.toml` for a list of possible drivers at the bottom of the
file. Each `driver-*` feature should have a comment explaining what it does.

Choose the driver you want to use and run
```bash
just build-release <driver>
```
or just `build` if you don't want release optimizations.

This will build the server binary and the client WASM.

TODO: Explain deploying server on RasPi or similar

## Adding an effect

Feel free to open a PR if you want to add a new effect!

To add a new effect, you only need to look at the `ww-effects` crate. The actual effect
implementations live in the `effects` module. You may want to create a new submodule for your
effect if you don't want to use the modules that already exist. You will need to add the actual
implementation for your effect, using the existing implementations as examples. You should have
```rust
use crate::effects::prelude::*;
```
at the top of the file, separate `config` and `effect` modules with feature-dependent public
exports, and preferably some tests at the end. You must also publicly export the effect from the
parent modules up to `src/effect`. You must also add the name of your effect to `src/lib.rs` like
so:
```rust
pub mod list {
    effect_proc_macros::generate_lists_and_impls! {
        // ...
        MyNewEffect,
    }
}
```

## Adding a driver

Feel free to open a PR if you want to add a new driver!

To add a new driver, you should first create a new crate in the `drivers/` directory and add it to the
workspace members in the root `Cargo.toml`. This crate must export a public type which implements
`ww_driver_trait::Driver` and has a public method of the following form:
```rust
impl MyNewDriver {
    pub fn init() -> Self {
        // ...
    }
}
```

All the implementation details of how the driver works are internal to the crate and unspecified.
If your driver crate uses anything which is already defined as a workspace dependency, then
please define it as such in the `Cargo.toml`.

To register your new driver, you will need to add it in a few places:
1. As a feature starting with `driver-` in `ww-server/Cargo.toml`
1. The `cfg_if` block in `ww-server/src/drivers/mod.rs`, following the pattern of the other drivers
1. `DRIVER_NAMES` in `ww-server/build.rs`, following the pattern
1. The `_check` recipe in the `justfile`
1. The `build` job in `.github/workflows/ci.yaml`
