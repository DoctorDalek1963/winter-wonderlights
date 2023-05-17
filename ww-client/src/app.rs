//! This module handles the [`App`] type for the `eframe`-based GUI.

use async_channel::{Receiver, Sender};
use reqwest::Client;
use std::time::Duration;
use strum::IntoEnumIterator;
use tracing::{debug, error, instrument};
use tracing_unwrap::ResultExt;
use ww_effects::EffectNameList;
use ww_shared::{ClientState, ClientToServerMsg, ServerToClientMsg};

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
