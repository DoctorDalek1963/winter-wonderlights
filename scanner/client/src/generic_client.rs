//! This module provides the [`GenericClientWidget`] type to be shared between
//! [`CameraWidget`](super::camera::CameraWidget) and
//! [`ControllerWidget`](super::controller::ControllerWidget).

use async_channel::{Receiver, Sender};
use ewebsock::{WsEvent, WsMessage, WsReceiver};
use prokio::time::sleep;
use serde::{Deserialize, Serialize};
use std::{
    fmt::Debug,
    sync::{Arc, RwLock},
    time::Duration,
};
use tracing::{debug, error, info, instrument};
use tracing_unwrap::ResultExt;
use ww_scanner_shared::{client_impl::ClientToServerMsg, GenericServerToClientMsg};

use crate::app::AppState;

/// A genericised state for a client.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GenericClientState {
    /// Waiting to connect to the server.
    WaitingForConnection,

    /// The server rejected the client because another client of the same type was already
    /// connected.
    Rejected,

    /// We're connected but the server is not yet ready to start scanning.
    ServerNotReady,

    /// We're connected and the server is ready to start scanning.
    ServerReady,
}

/// A generic client widget that can talk to the server in its own background tasks, negotiate
/// reconnects, etc.
///
/// `CSM` is the type of messages from the client to the server, and `SCM` is the type of messages
/// from the server to the client.
#[derive(Clone, Debug)]
pub struct GenericClientWidget<CSM, SCM> {
    /// Receive messages from the server.
    pub server_rx: Receiver<SCM>,

    /// Send messages to the server.
    pub message_tx: Sender<CSM>,

    /// Tell all background tasks for this widget to reconnect to server.
    pub reconnect_tx: async_broadcast::Sender<()>,

    /// The generic state of the client.
    pub state: Arc<RwLock<GenericClientState>>,

    /// An async runtime used to send async messages.
    pub async_runtime: prokio::Runtime,
}

impl<CSM, SCM> GenericClientWidget<CSM, SCM>
where
    CSM: Clone + Debug + Send + Serialize + ClientToServerMsg + 'static,
    SCM: Debug + Send + for<'de> Deserialize<'de> + 'static,
{
    /// Create a new [`GenericClientWidget`] and initialise background tasks.
    pub fn new(
        async_runtime: prokio::Runtime,
        make_establish_connection_message: impl FnOnce() -> CSM,
    ) -> Self {
        let (server_tx, server_rx) = async_channel::unbounded();
        let (message_tx, message_rx) = async_channel::unbounded();

        let (reconnect_tx, reconnect_rx) = async_broadcast::broadcast(1);

        // Set up server communication
        let (ws_receiver_oneshot_tx, ws_receiver_oneshot_rx) = async_channel::bounded(1);

        async_runtime.spawn_pinned({
            let reconnect_rx = reconnect_rx.clone();
            move || Self::send_messages_to_server(message_rx, ws_receiver_oneshot_tx, reconnect_rx)
        });
        async_runtime.spawn_pinned({
            let reconnect_rx = reconnect_rx.clone();
            move || {
                Self::receive_messages_from_server(server_tx, ws_receiver_oneshot_rx, reconnect_rx)
            }
        });

        let state = Arc::new(RwLock::new(GenericClientState::WaitingForConnection));

        let establish_connection_message = make_establish_connection_message();

        // Try to establish connection, then loop and constantly check to see if we need to
        // reconnect
        async_runtime.spawn_pinned({
            let message_tx = message_tx.clone();
            let mut reconnect_rx = reconnect_rx;
            let reconnect_tx = reconnect_tx.clone();
            let state = state.clone();

            move || async move {
                Self::send_establish_connection(message_tx.clone(), establish_connection_message.clone()).await;

                loop {
                    sleep(Duration::from_secs(1)).await;

                    if reconnect_rx.try_recv().is_ok() {
                        Self::send_establish_connection(message_tx.clone(), establish_connection_message.clone()).await;
                        continue;
                    }

                    // This is done to ensure the lock isn't held across an await point
                    let reconnect = matches!(state.try_read(), Ok(state) if *state == GenericClientState::WaitingForConnection);
                    if reconnect {
                        reconnect_tx
                            .broadcast(())
                            .await
                            .expect_or_log("Should be able to send () down reconnect channel");
                    }
                }
            }
        });

        Self {
            server_rx,
            message_tx,
            reconnect_tx,
            state,
            async_runtime,
        }
    }

    /// Send the `DeclareClientType` message to the server and then the `EstablishConnection`
    /// message.
    #[instrument(skip_all)]
    async fn send_establish_connection(message_tx: Sender<CSM>, establish_connection_message: CSM) {
        info!("Trying to connect to server");

        match message_tx
            .send(CSM::make_declare_client_type_message())
            .await
        {
            Ok(()) => (),
            Err(e) => {
                error!(failed_message = ?e.into_inner(), "Error sending DeclareClientType message on channel");
            }
        };

        sleep(Duration::from_millis(250)).await;

        match message_tx.send(establish_connection_message).await {
            Ok(()) => (),
            Err(e) => {
                error!(failed_message = ?e.into_inner(), "Error sending EstablishConnection message on channel");
            }
        };
    }

    /// Recieve [`ClientToServerMsg`]s on the channel and send them to the server.
    ///
    /// This function also connects to the server using WebSockets and sends the [`WsReceiver`]
    /// down the given channel so that [`Self::receive_messages_from_server`] can listen to the server.
    #[instrument(skip_all)]
    async fn send_messages_to_server(
        rx: Receiver<CSM>,
        ws_receiver_tx: Sender<WsReceiver>,
        mut reconnect_rx: async_broadcast::Receiver<()>,
    ) {
        loop {
            let (mut ws_sender, ws_receiver) = ewebsock::connect(
                std::env::var("SCANNER_SERVER_URL")
                    .expect_or_log("SCANNER_SERVER_URL must be defined"),
            )
            .expect_or_log("Should be able to use WebSockets");

            match ws_receiver_tx.send(ws_receiver).await {
                Ok(()) => (),
                Err(error) => {
                    error!(?error, "Failed to send WsReceiver down channel");
                    panic!("Failed to send WsReceiver down channel");
                }
            };

            sleep(Duration::from_millis(500)).await;

            loop {
                if reconnect_rx.try_recv().is_ok() {
                    // Try to connect again with ewebsock::connect() at the top of this function
                    debug!("Trying to reconnect to server");
                    drop(ws_sender);
                    break;
                }

                //while let Ok(msg) = rx.recv().await {
                if let Ok(msg) = rx.try_recv() {
                    debug!(?msg, "Sending message to server");

                    let bytes_to_send = if let Some(bytes) = msg.is_declare_client_type_message() {
                        debug!("Sending DeclareClientType");
                        Vec::from(bytes)
                    } else {
                        bincode::serialize(&msg).expect_or_log("Should be able to serialize a CSM")
                    };

                    ws_sender.send(WsMessage::Binary(bytes_to_send));
                } else {
                    sleep(Duration::from_millis(10)).await;
                }
            }
        }
    }

    /// Receive `SCM`s over the internet and send them down the channel so that
    /// `respond_to_server_messages` can respond to them.
    ///
    /// See
    /// [`CameraWidget::respond_to_server_messages`](super::camera::CameraWidget::respond_to_server_messages) and
    /// [`ControllerWidget::respond_to_server_messages`](super::controller::ControllerWidget::respond_to_server_messages).
    #[instrument(skip_all)]
    async fn receive_messages_from_server(
        tx: Sender<SCM>,
        ws_receiver_rx: Receiver<WsReceiver>,
        mut reconnect_rx: async_broadcast::Receiver<()>,
    ) {
        loop {
            let ws_receiver = ws_receiver_rx
                .recv()
                .await
                .expect_or_log("Should be able to receive WsReceiver down channel");

            loop {
                if reconnect_rx.try_recv().is_ok() {
                    // Try to receive a new ws_receiver
                    debug!("Trying to reconnect to server");
                    drop(ws_receiver);
                    break;
                }

                // Try to receive a message over the WebSocket and send it to `respond_to_server_messages`
                if let Some(WsEvent::Message(raw_msg)) = ws_receiver.try_recv() {
                    debug!(?raw_msg, "Received raw message from server");
                    match raw_msg {
                        WsMessage::Binary(bytes) => {
                            let msg: SCM = bincode::deserialize(&bytes)
                                .expect_or_log("Failed to deserialize bytes of message");
                            debug!(?msg, "Deserialized message from server");
                            tx.send(msg)
                                .await
                                .expect_or_log("Should be able to send SCM down channel");
                        }
                        raw_msg => error!(
                            ?raw_msg,
                            "Unexpected WebSocket message type - we only expect binary messages"
                        ),
                    }
                } else {
                    sleep(Duration::from_millis(10)).await;
                }
            }
        }
    }

    /// Respond to a generic server message by setting the [`GenericClientState`] and returning the
    /// new state for the top level [`App`](super::app::App).
    pub fn respond_to_generic_server_message(&mut self, msg: GenericServerToClientMsg) -> AppState {
        use GenericClientState as State;
        use GenericServerToClientMsg as Msg;

        let mut state = self
            .state
            .write()
            .expect_or_log("Should be able to write to client widget state");

        match msg {
            Msg::AcceptConnection | Msg::ServerNotReady => {
                *state = State::ServerNotReady;
                AppState::Connected
            }
            Msg::RejectConnection => {
                *state = State::Rejected;
                AppState::Rejected
            }
            Msg::TerminateConnection => {
                *state = State::WaitingForConnection;
                let reconnect_tx = self.reconnect_tx.clone();
                self.async_runtime.spawn_pinned(move || async move {
                    reconnect_tx
                        .broadcast(())
                        .await
                        .expect_or_log("Should be able to send () down reconnect channel");
                });
                AppState::Rejected
            }
            Msg::ServerReady => {
                *state = State::ServerReady;
                AppState::Connected
            }
        }
    }

    /// Send a message to the server by spawning a task on the async runtime.
    pub fn send_msg(&self, msg: CSM) {
        self.async_runtime.spawn_pinned({
            let tx = self.message_tx.clone();
            || async move {
                tx.send(msg).await.unwrap_or_log();
            }
        });
    }
}
