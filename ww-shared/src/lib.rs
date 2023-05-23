//! This crate handles messages sent between the server and the client.

use serde::{Deserialize, Serialize};
use std::fs;
use ww_effects::list::{EffectConfigDispatchList, EffectNameList};

/// A message from the server to the client.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ServerToClientMsg {
    /// Tell the client to update to the new state.
    UpdateClientState(ClientState),
}

/// A message from the client to the server.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ClientToServerMsg {
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
        let _ = fs::DirBuilder::new()
            .recursive(true)
            .create(format!("{}/config", env!("DATA_DIR")));

        let write_and_return_default = || -> Self {
            let default = Self::default();
            default.save_to_file(filename);
            default
        };

        let Ok(text) = fs::read_to_string(format!("{}/config/{filename}", env!("DATA_DIR"))) else {
            return write_and_return_default();
        };

        ron::from_str(&text).unwrap_or_else(|_| write_and_return_default())
    }

    /// Save the client to a file.
    pub fn save_to_file(&self, filename: &str) {
        let _ = fs::write(
            format!("{}/config/{filename}", env!("DATA_DIR")),
            ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default().struct_names(true))
                .expect("ClientState should be serializable"),
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
