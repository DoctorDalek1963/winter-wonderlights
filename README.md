# Winter WonderLights

This is a Rust project designed to display 3D effects on a Christmas tree using individually addressable RGB LEDs. It was directly inspired by [this video by Matt Parker](https://www.youtube.com/watch?v=TvlpIojusBE).

## Quickstart

#### Pre-requisites

To run Winter WonderLights, you will first need a small computer like a Raspberry Pi. You will then need to setup a web server like [nginx](https://nginx.org/en/) or [Apache](https://httpd.apache.org/) on it and choose a port number for the Winter WonderLights server. Any port number between 10000 and 65535 will do. Make sure to allow this port as well as ports 80 and 443 through the firewall on the Raspberry Pi. You will also need to port-forward these 3 ports if you want to allow other people to access the server without connecting to the Wi-Fi.

You will also need encryption in the form of an SSL certificate/key pair. You can get this for free from [Let's Encrypt](https://letsencrypt.org/) and use `certbot` to keep it up to date automatically.

If you want the server to be public, then you will also need a fixed DNS address, which you can get for free from [No-IP](https://www.noip.com/) (make sure to setup the DUC properly). If you only want people to access the server from your home Wi-Fi, then you'll just need the IP address of the Raspberry Pi.

#### The `.env` file

You need a file called `.env` in the root of the project folder.

If you just want to use the tree, the `.env` file should look like this:
```bash
# Server
export DATA_DIR=/path/to/winter/wonderlights/data
export COORDS_FILENAME=coords-filename.gift

export SERVER_SSL_CERT_PATH=/path/to/ssl/certificate.pem
export SERVER_SSL_KEY_PATH=/path/to/ssl/privatekey.pem

export PORT=23120
export LIGHTS_NUM=250

# Client
export SERVER_URL=wss://my.server.net:${PORT}

# Scanner server
export SCANNER_PORT=23121

# Scanner clients
export SCANNER_SERVER_URL=wss://my.server.net:${SCANNER_PORT}
```

If you're just using the project at home and all clients will be on your home Wi-Fi, then you can use the local IP of the server (Raspberry Pi) instead of a DNS address for the `SERVER_URL`.

If you want to develop Winter WonderLights, the `.env` file should look like this:
```bash
# Server
export DATA_DIR=/path/to/project/folder/data
export COORDS_FILENAME=2020-matt-parker.gift

export SERVER_SSL_CERT_PATH=/dev/null
export SERVER_SSL_KEY_PATH=/dev/null

export PORT=23120
export LIGHTS_NUM=250

# Client
export SERVER_URL=ws://localhost:${PORT}

# Scanner server
export SCANNER_PORT=23121

# Scanner clients
export SCANNER_SERVER_URL=ws://localhost:${SCANNER_PORT}
```

#### Dependencies

To compile the program yourself, you will need [Rust](https://rustup.rs/) and you will need to install a few things with `cargo`. I highly recommend using `cargo binstall`, which you can install with
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

If you're setting up for development, also install `cargo-insta` the same way.

#### Compiling the program

Winter WonderLights uses a system of different drivers to decide how the effects get displayed. See `ww-server/Cargo.toml` for a list of possible drivers at the bottom of the file. Each `driver-*` feature should have a comment explaining what it does.

Choose the driver you want to use and run
```bash
just build-release <driver>
```
or just `build` if you don't want release optimizations.

This will build the server binary and the client WASM.

TODO: Explain deploying server on RasPi or similar

## Adding an effect

Feel free to open a PR if you want to add a new effect!

To add a new effect, you only need to look at the `ww-effects` crate. The actual effect implementations live in the `effects` module. You may want to create a new submodule for your effect if you don't want to use the modules that already exist. You will need to add the actual implementation for your effect, using the existing implementations as examples. Your new effect file should look something like this (but everything should be properly documented):
```rust
#[cfg(feature = "config-impls")]
pub use config::MyNewEffectConfig;

#[cfg(feature = "effect-impls")]
pub use effect::MyNewEffect;

use crate::effects::prelude::*;

#[cfg(feature = "config-impls")]
mod config {
    use super::*;

    #[derive(Clone, PartialEq, Serialize, Deserialize, BaseEffectConfig)]
    pub struct MyNewEffectConfig {
        // ...
    }

    impl Default for MyNewEffectConfig {
        // ...
    }

    impl EffectConfig for MyNewEffectConfig {
        fn render_options_gui(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) -> bool {
            // ...
        }
    }
}

#[cfg(feature = "effect-impls")]
mod effect {
    use super::*;

    #[derive(BaseEffect)]
    pub struct MyNewEffect {
        config: MyNewEffectConfig,
        // ...
    }

    impl Effect for MyNewEffect {
        fn from_config(config: MyNewEffectConfig) -> Self {
            // ...
        }

        async fn run(self, driver: &mut dyn Driver) {
            // ...
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{traits::Effect, TestDriver};

    #[tokio::test]
    async fn my_new_effect_test() {
        // ...
    }
}
```

You must also publicly export the effect from the parent modules up to `src/effect`. You must also add the name of your effect to `src/lib.rs` like so:
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

To add a new driver, you should first create a new crate in the `drivers/` directory and add it to the workspace members in the root `Cargo.toml`. This crate must export a public type which implements `ww_driver_trait::Driver`.

All the implementation details of how the driver works are internal to the crate and unspecified. If your driver crate uses anything which is already defined as a workspace dependency, then please define it as such in the `Cargo.toml`.

To register your new driver, you will need to add it in a few places:
1. As a feature starting with `driver-` in `ww-server/Cargo.toml`
1. The `cfg_if` block in `ww-server/src/drivers/mod.rs`, following the pattern of the other drivers
1. `DRIVER_NAMES` in `ww-server/build.rs`, following the pattern
1. The `_check` recipe in the `justfile`
1. The `build` job in `.github/workflows/ci.yaml`
