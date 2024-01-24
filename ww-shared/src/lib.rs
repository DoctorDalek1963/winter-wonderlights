//! This crate handles messages sent between the server and the client.

#![feature(lint_reasons)]

use serde::{Deserialize, Serialize};
use std::fs;
use tracing_unwrap::ResultExt;
use ww_effects::list::{EffectConfigDispatchList, EffectNameList};

/// The version of this crate.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// A message from the server to the client.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ServerToClientMsg {
    /// Establish a connection with the client by agreeing on a protocol version and specifying the
    /// server version for the client's `About` panel.
    ///
    /// The protocol version is [`CRATE_VERSION`] and the server version is
    /// [`ww_server::CRATE_VERSION`](../ww_server/const.CRATE_VERSION.html).
    EstablishConnection {
        /// The version number of the protocol, which is the version of this crate that the server
        /// was compiled with.
        protocol_version: String,

        /// The version number of the server binary.
        server_version: String,
    },

    /// Like [`EstablishConnection`](Self::EstablishConnection), but we deny the client based on a protocol version mismatch.
    DenyConnection {
        /// The version number of the protocol, which is the version of this crate that the server
        /// was compiled with.
        protocol_version: String,

        /// The version number of the server binary.
        server_version: String,
    },

    /// Tell the client to update to the new state.
    UpdateClientState(ClientState),

    /// Terminate the connection between the server and the client.
    TerminateConnection,
}

/// A message from the client to the server.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ClientToServerMsg {
    /// Establish a connection with the server by agreeing on a protocol version.
    ///
    /// The protocol version is [`CRATE_VERSION`].
    EstablishConnection {
        /// The version number of the protocol, which is the version of this crate that the client
        /// was compiled with.
        protocol_version: String,
    },

    /// Request an [`UpdateClientState`](ServerToClientMsg::UpdateClientState) message from the server.
    RequestUpdate,

    /// Update the config to the one specified.
    UpdateConfig(EffectConfigDispatchList),

    /// Ask the server to change the effect.
    ChangeEffect(Option<EffectNameList>),

    /// Restart the current effect.
    RestartCurrentEffect,
}

/// The state of the client.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClientState {
    /// The name of the current effect.
    pub effect_name: Option<EffectNameList>,

    /// The config of the current effect.
    pub effect_config: Option<EffectConfigDispatchList>,
}

impl ClientState {
    /// Load the client state from a file.
    pub fn from_file(filename: &str) -> Self {
        let _ = fs::DirBuilder::new().recursive(true).create(format!(
            "{}/config",
            std::env::var("DATA_DIR").expect_or_log("DATA_DIR must be defined")
        ));

        let write_and_return_default = || -> Self {
            let default = Self::default();
            default.save_to_file(filename);
            default
        };

        let Ok(text) = fs::read_to_string(format!(
            "{}/config/{filename}",
            std::env::var("DATA_DIR").expect_or_log("DATA_DIR must be defined")
        )) else {
            return write_and_return_default();
        };

        ron::from_str(&text).unwrap_or_else(|_| write_and_return_default())
    }

    /// Save the client to a file.
    pub fn save_to_file(&self, filename: &str) {
        let _ = fs::write(
            format!(
                "{}/config/{filename}",
                std::env::var("DATA_DIR").expect_or_log("DATA_DIR must be defined")
            ),
            ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default().struct_names(true))
                .expect_or_log("ClientState should be serializable"),
        );
    }
}

impl Default for ClientState {
    fn default() -> Self {
        Self {
            effect_name: None,
            effect_config: None,
        }
    }
}
