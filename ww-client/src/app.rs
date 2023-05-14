//! This module handles the [`App`] type for the `eframe`-based GUI.

use async_channel::{Receiver, Sender};
use reqwest::Client;
use std::time::Duration;
use tracing::{debug, error, instrument};
use tracing_unwrap::ResultExt;
use ww_shared_msgs::{ClientState, ClientToServerMsg, ServerToClientMsg};

/// The `.expect()` error message for serializing a [`ClientToServerMsg`].
const EXPECT_SERIALIZE_MSG: &str = "Serializing a ClientToServerMsg should never fail";

/// The app type itself.
pub struct App {
    /// The receiver end of a channel for [`ServerToClientMsg`].
    server_rx: Receiver<ServerToClientMsg>,

    /// The sender end of a channel used to send messages to the server.
    message_tx: Sender<ClientToServerMsg>,

    /// The state of the client.
    state: Option<ClientState>,

    /// An async runtime used to send async messages.
    async_runtime: prokio::Runtime,
}

impl App {
    /// Create a new [`App`] and initialise sound background processes.
    pub fn new(_cc: &eframe::CreationContext) -> Self {
        let (server_tx, server_rx) = async_channel::unbounded();
        let (message_tx, message_rx) = async_channel::unbounded();
        let client = Client::new();

        prokio::Runtime::default().spawn_pinned({
            let message_tx = message_tx.clone();
            move || async move {
                loop {
                    match message_tx.send(ClientToServerMsg::RequestUpdate).await {
                        Ok(()) => (),
                        Err(e) => error!(?e, "Error sending message down channel"),
                    }
                    prokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        });
        prokio::Runtime::default()
            .spawn_pinned(move || Self::send_messages(client, message_rx, server_tx));

        Self {
            server_rx,
            message_tx,
            state: None,
            async_runtime: prokio::Runtime::default(),
        }
    }

    /// Recieve [`ClientToServerMsg`]s on the channel and send them to the server.
    #[instrument(skip_all)]
    async fn send_messages(
        client: Client,
        rx: Receiver<ClientToServerMsg>,
        tx: Sender<ServerToClientMsg>,
    ) {
        while let Ok(msg) = rx.recv().await {
            match client
                .post(env!("SERVER_URL"))
                .body(ron::to_string(&msg).expect_or_log(EXPECT_SERIALIZE_MSG))
                .send()
                .await
            {
                Ok(response) => match response.text().await {
                    Ok(body) => match ron::from_str(&body) {
                        Ok(msg) => match tx.send(msg).await {
                            Ok(()) => (),
                            Err(e) => error!(?e, "Error sending message down channel"),
                        },
                        Err(e) => error!(?e, "Error deserializing message from server"),
                    },
                    Err(e) => error!(?e, "Error getting text from response"),
                },
                Err(e) => error!(?e, "Error communicating with server"),
            }
        }
    }

    /// Respond to all the server messages on `self.server_rx`.
    #[instrument(skip_all)]
    fn respond_to_server_messages(&mut self) {
        while let Ok(msg) = self.server_rx.try_recv() {
            debug!(?msg, "Responding to server message");

            match msg {
                ServerToClientMsg::UpdateClientState(state) => self.state = Some(state),
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        self.respond_to_server_messages();

        egui::CentralPanel::default().show(ctx, |ui| match &mut self.state {
            None => {
                ui.centered_and_justified(|ui| ui.spinner());
            }
            Some(state) => {
                if let Some(config) = &mut state.effect_config {
                    if config.render_options_gui(ctx.into(), ui) {
                        debug!("Config changed, sending message");

                        // Since we're now using async_channel, we need to send the message in an
                        // async runtime
                        self.async_runtime.spawn_pinned({
                            let message_tx = self.message_tx.clone();
                            let config = config.clone();

                            move || async move {
                                message_tx
                                    .send(ClientToServerMsg::UpdateConfig(config))
                                    .await
                                    .expect_or_log(
                                        "Unable to send UpdateConfig message down channel",
                                    );
                            }
                        })
                    }
                }
            }
        });

        // We need to constantly be repainting the GUI so that new server messages are always
        // processed.
        ctx.request_repaint();
    }
}
