//! This crate handles messages sent between the server and the client.

use serde::{Deserialize, Serialize};
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
}

/// The state of the client.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClientState {
    /// The name of the current effect.
    pub effect_name: Option<EffectNameList>,

    /// The config of the current effect.
    pub effect_config: Option<EffectConfigDispatchList>,
}
