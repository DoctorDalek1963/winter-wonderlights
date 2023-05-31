//! This module handles the [`App`] type for the `eframe`-based GUI.

use async_channel::{Receiver, Sender};
use ewebsock::{WsEvent, WsMessage, WsReceiver};
use futures_channel::oneshot;
use prokio::time::sleep;
use std::time::Duration;
use strum::IntoEnumIterator;
use tracing::{debug, error, instrument};
use tracing_unwrap::ResultExt;
use ww_effects::EffectNameList;
use ww_shared::{ClientState, ClientToServerMsg, ServerToClientMsg};

/// The app type itself.
pub struct App {
    /// The receiver end of a channel used to recieve messages from the server.
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

        // Get initial state
        prokio::Runtime::default().spawn_pinned({
            let message_tx = message_tx.clone();
            move || async move {
                prokio::time::sleep(Duration::from_millis(500)).await;
                match message_tx.send(ClientToServerMsg::RequestUpdate).await {
                    Ok(()) => (),
                    Err(e) => error!(?e, "Error sending message down channel"),
                }
            }
        });

        let (ws_receiver_oneshot_tx, ws_receiver_oneshot_rx) = oneshot::channel();

        prokio::Runtime::default()
            .spawn_pinned(move || Self::send_messages(message_rx, ws_receiver_oneshot_tx));
        prokio::Runtime::default()
            .spawn_pinned(move || Self::receive_messages(server_tx, ws_receiver_oneshot_rx));

        Self {
            server_rx,
            message_tx,
            state: None,
            async_runtime: prokio::Runtime::default(),
        }
    }

    /// Recieve [`ClientToServerMsg`]s on the channel and send them to the server.
    ///
    /// This function also connects to the server using WebSockets and sends the [`WsReceiver`]
    /// down the given oneshot channel so that [`Self::receive_messages`] can listen to the server.
    #[instrument(skip_all)]
    async fn send_messages(
        rx: Receiver<ClientToServerMsg>,
        ws_receiver_oneshot_tx: oneshot::Sender<WsReceiver>,
    ) {
        let (mut ws_sender, ws_receiver) =
            ewebsock::connect(env!("SERVER_URL")).expect_or_log("Should be able to use WebSockets");

        match ws_receiver_oneshot_tx.send(ws_receiver) {
            Ok(()) => (),
            Err(_ws_receiver) => {
                error!("Failed to send WsReceiver down channel");
                panic!("Failed to send WsReceiver down channel");
            }
        };

        while let Ok(msg) = rx.recv().await {
            debug!(?msg, "Sending message to server");
            ws_sender
                .send(WsMessage::Binary(bincode::serialize(&msg).expect_or_log(
                    "Should be able to serialize a ClientToServerMsg",
                )));
        }
    }

    /// Receive [`ServerToClientMsg`]s over the internet and send them down the channel so that
    /// [`Self::respond_to_server_messages`] can respond to them.
    #[instrument(skip_all)]
    async fn receive_messages(
        tx: Sender<ServerToClientMsg>,
        ws_receiver_oneshot_rx: oneshot::Receiver<WsReceiver>,
    ) {
        let ws_receiver = ws_receiver_oneshot_rx
            .await
            .expect_or_log("Should be able to receive WsReceiver down channel");

        loop {
            if let Some(WsEvent::Message(msg)) = ws_receiver.try_recv() {
                match msg {
                    WsMessage::Binary(bytes) => {
                        let msg: ServerToClientMsg = bincode::deserialize(&bytes)
                            .expect_or_log("Failed to deserialize bytes of message");
                        tx.send(msg)
                            .await
                            .expect_or_log("Should be able to send ServerToClientMsg down channel");
                    }
                    msg => error!(
                        ?msg,
                        "Unexpected WebSocket message type - we only expect binary messages"
                    ),
                }
            } else {
                sleep(Duration::from_millis(10)).await;
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
                let new_effect_selected = egui::ComboBox::from_label("Current effect")
                    .selected_text(
                        state
                            .effect_name
                            .map_or("None", |effect| effect.effect_name()),
                    )
                    .show_ui(ui, |ui| {
                        let selected_none = ui
                            .selectable_value(&mut state.effect_name, None, "None")
                            .clicked();

                        let selected_new_effect = EffectNameList::iter().any(|effect| {
                            // We remember which value was initially selected and whether this
                            // value is a new one
                            let different = Some(effect) != state.effect_name;
                            let resp = ui.selectable_value(
                                &mut state.effect_name,
                                Some(effect),
                                effect.effect_name(),
                            );

                            // If the value is different from the old and has been clicked, then we care
                            resp.clicked() && different
                        });

                        if selected_new_effect || selected_none {
                            Some(state.effect_name.clone())
                        } else {
                            None
                        }
                    })
                    .inner
                    //.flatten()
                    .flatten();

                let restart_effect = ui.button("Restart current effect").clicked();

                let effect_config_changed = if let Some(config) = &mut state.effect_config {
                    ui.separator();
                    config
                        .render_options_gui(ctx.into(), ui)
                        .then_some(config.clone())
                } else {
                    None
                };

                // TODO: Collapse these cases into a single `self.async_runtime.spawn_pinned()`
                // call to reduce overhead
                if let Some(name) = new_effect_selected {
                    debug!("New effect selected, sending message");

                    self.async_runtime.spawn_pinned({
                        let message_tx = self.message_tx.clone();
                        let name = name.clone();

                        move || async move {
                            message_tx
                                .send(ClientToServerMsg::ChangeEffect(name))
                                .await
                                .expect_or_log("Unable to send UpdateConfig message down channel");
                        }
                    });
                }

                if restart_effect {
                    debug!("Restarting current effect");

                    self.async_runtime.spawn_pinned({
                        let message_tx = self.message_tx.clone();

                        move || async move {
                            message_tx
                                .send(ClientToServerMsg::RestartCurrentEffect)
                                .await
                                .expect_or_log(
                                    "Unable to send RestartCurrentEffect message down channel",
                                );
                        }
                    });
                }

                if let Some(config) = effect_config_changed {
                    debug!("Effect config changed, sending message");

                    self.async_runtime.spawn_pinned({
                        let message_tx = self.message_tx.clone();
                        let config = config.clone();

                        move || async move {
                            message_tx
                                .send(ClientToServerMsg::UpdateConfig(config))
                                .await
                                .expect_or_log("Unable to send UpdateConfig message down channel");
                        }
                    });
                }
            }
        });

        // We need to constantly be repainting the GUI so that new server messages are always
        // processed.
        ctx.request_repaint();
    }
}
