//! This crate handles messages sent between the server and the client.

use serde::{Deserialize, Serialize};
use ww_effects::EffectNameList;

/// A message from the server to the client.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerToClientMsg {
    UpdateClientState,
}

/// A message from the client to the server.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientToServerMsg {
    RequestUpdate,
    ChangeEffect(EffectNameList),
}
