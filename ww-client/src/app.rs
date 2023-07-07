//! This module handles the [`App`] type for the `eframe`-based GUI.

use async_channel::{Receiver, Sender};
use egui::RichText;
use ewebsock::{WsEvent, WsMessage, WsReceiver};
use prokio::time::sleep;
use std::{
    sync::{Arc, RwLock},
    time::Duration,
};
use strum::IntoEnumIterator;
use tracing::{debug, error, info, instrument, trace, warn};
use tracing_unwrap::ResultExt;
use ww_effects::EffectNameList;
use ww_shared::{ClientState, ClientToServerMsg, ServerToClientMsg};

/// The current state of the app and its connection to the server.
#[derive(Clone, Debug, PartialEq)]
enum AppState {
    /// We're currently waiting to connect to the server. [`ServerToClientMsg::EstablishConnection`] has not yet been received.
    WaitingForConnection,

    /// We're connected to the server and we're holding on to some [`ClientState`].
    Connected {
        /// The current [`ClientState`] of the app.
        state: ClientState,

        /// The version of the server that we're connected to.
        server_version: String,
    },

    /// The client and server use different versions of the communications protocol (the
    /// [`ww_shared`](../../ww_shared/index.html) crate), so we can't communicate.
    ProtocolMismatch {
        /// The version of the server that rejected us.
        server_version: String,

        /// The version of the protocol that the server is using.
        server_protocol_version: String,
    },
}

/// The app type itself.
pub struct App {
    /// The receiver end of a channel used to recieve messages from the server.
    server_rx: Receiver<ServerToClientMsg>,

    /// The sender end of a channel used to send messages to the server.
    message_tx: Sender<ClientToServerMsg>,

    /// The transmitter end of a broadcast channel which can signal when to try to reconnect to the
    /// server.
    reconnect_tx: async_broadcast::Sender<()>,

    /// The state of the client.
    state: Arc<RwLock<AppState>>,

    /// An async runtime used to send async messages.
    async_runtime: prokio::Runtime,

    // This is used only by [`App::respond_to_server_messages`].
    #[doc(hidden)]
    tracked_server_version: Option<String>,
}

impl App {
    /// Create a new [`App`] and initialise sound background processes.
    pub fn new(_cc: &eframe::CreationContext) -> Self {
        let (server_tx, server_rx) = async_channel::unbounded();
        let (message_tx, message_rx) = async_channel::unbounded();

        let (reconnect_tx, reconnect_rx) = async_broadcast::broadcast(1);

        let state = Arc::new(RwLock::new(AppState::WaitingForConnection));
        let async_runtime = prokio::Runtime::default();

        // Connect to the server
        async_runtime.spawn_pinned({
            let message_tx = message_tx.clone();
            let mut reconnect_rx = reconnect_rx.clone();
            let reconnect_tx = reconnect_tx.clone();
            let state = state.clone();

            move || async move {
                Self::send_establish_connection(message_tx.clone()).await;

                loop {
                    if reconnect_rx.try_recv().is_ok() {
                        Self::send_establish_connection(message_tx.clone()).await;
                        continue;
                    }

                    // This is done to ensure the lock isn't held across an await point
                    let reconnect = matches!(state.try_read(), Ok(state) if *state == AppState::WaitingForConnection);
                    if reconnect {
                        reconnect_tx
                            .broadcast(())
                            .await
                            .expect_or_log("Should be able to send () down reconnect channel");
                    }

                    sleep(Duration::from_secs(1)).await;
                }
            }
        });

        let (ws_receiver_oneshot_tx, ws_receiver_oneshot_rx) = async_channel::bounded(1);

        async_runtime.spawn_pinned({
            let reconnect_rx = reconnect_rx.clone();
            move || Self::send_messages_to_server(message_rx, ws_receiver_oneshot_tx, reconnect_rx)
        });
        async_runtime.spawn_pinned({
            move || {
                Self::receive_messages_from_server(server_tx, ws_receiver_oneshot_rx, reconnect_rx)
            }
        });

        Self {
            server_rx,
            message_tx,
            reconnect_tx,
            state,
            async_runtime,
            tracked_server_version: None,
        }
    }

    /// Send the [`ClientToServerMsg::EstablishConnection`] message down the channel.
    #[instrument(skip_all)]
    async fn send_establish_connection(tx: Sender<ClientToServerMsg>) {
        info!("Sending EstablishConnection down channel");

        match tx
            .send(ClientToServerMsg::EstablishConnection {
                protocol_version: ww_shared::CRATE_VERSION.to_string(),
            })
            .await
        {
            Ok(()) => (),
            Err(e) => error!(?e, "Error sending message down channel"),
        };
    }

    /// Recieve [`ClientToServerMsg`]s on the channel and send them to the server.
    ///
    /// This function also connects to the server using WebSockets and sends the [`WsReceiver`]
    /// down the given channel so that [`Self::receive_messages_from_server`] can listen to the server.
    #[instrument(skip_all)]
    async fn send_messages_to_server(
        rx: Receiver<ClientToServerMsg>,
        ws_receiver_tx: Sender<WsReceiver>,
        mut reconnect_rx: async_broadcast::Receiver<()>,
    ) {
        loop {
            let (mut ws_sender, ws_receiver) = ewebsock::connect(env!("SERVER_URL"))
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
                    ws_sender
                        .send(WsMessage::Binary(bincode::serialize(&msg).expect_or_log(
                            "Should be able to serialize a ClientToServerMsg",
                        )));
                } else {
                    sleep(Duration::from_millis(10)).await;
                }
            }
        }
    }

    /// Receive [`ServerToClientMsg`]s over the internet and send them down the channel so that
    /// [`Self::respond_to_server_messages`] can respond to them.
    #[instrument(skip_all)]
    async fn receive_messages_from_server(
        tx: Sender<ServerToClientMsg>,
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
                if let Some(WsEvent::Message(msg)) = ws_receiver.try_recv() {
                    match msg {
                        WsMessage::Binary(bytes) => {
                            let msg: ServerToClientMsg = bincode::deserialize(&bytes)
                                .expect_or_log("Failed to deserialize bytes of message");
                            tx.send(msg).await.expect_or_log(
                                "Should be able to send ServerToClientMsg down channel",
                            );
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
    }

    /// Respond to all the server messages on `self.server_rx`.
    #[instrument(skip_all)]
    fn respond_to_server_messages(&mut self) {
        while let Ok(msg) = self.server_rx.try_recv() {
            debug!(?msg, "Responding to server message");

            match msg {
                ServerToClientMsg::EstablishConnection { server_version, .. } => {
                    self.tracked_server_version = Some(server_version);
                }

                ServerToClientMsg::DenyConnection {
                    protocol_version,
                    server_version,
                } => {
                    error!(
                        client_protocol_version = ?ww_shared::CRATE_VERSION,
                        server_protocol_version = ?protocol_version,
                        ?server_version,
                        "Protocol version mismatch"
                    );
                    *self.state.write().unwrap_or_log() = AppState::ProtocolMismatch {
                        server_version,
                        server_protocol_version: protocol_version,
                    };
                }

                ServerToClientMsg::UpdateClientState(state) => {
                    if let Some(server_version) = &self.tracked_server_version {
                        *self.state.write().unwrap_or_log() = AppState::Connected {
                            state,
                            server_version: server_version.clone(),
                        }
                    } else {
                        warn!("Received UpdateClientState before EstablishConnection; establishing new connection");

                        let message_tx = self.message_tx.clone();
                        self.async_runtime.spawn_pinned(|| async move {
                            message_tx
                                .send(ClientToServerMsg::EstablishConnection {
                                    protocol_version: ww_shared::CRATE_VERSION.to_string(),
                                })
                                .await
                                .expect("Unable to send EstablishConnection message down channel");
                        });
                    }
                }
                ServerToClientMsg::TerminateConnection => {
                    *self.state.write().unwrap_or_log() = AppState::WaitingForConnection;

                    let reconnect_tx = self.reconnect_tx.clone();
                    self.async_runtime.spawn_pinned(move || async move {
                        reconnect_tx
                            .broadcast(())
                            .await
                            .expect_or_log("Should be able to send () down reconnect channel");
                    });
                }
            }
        }
    }

    /// Display the GUI for waiting for a connection to the server.
    #[instrument(skip_all)]
    fn display_gui_waiting_for_connection(&mut self, ctx: &eframe::egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.centered_and_justified(|ui| ui.spinner());
        });
    }

    /// Display the GUI for being connected to the server. This method assumes that [`Self::state`]
    /// is [`AppState::Connected`] and will panic if it's not.
    #[instrument(skip_all)]
    fn display_gui_connected(&mut self, ctx: &eframe::egui::Context) {
        let AppState::Connected {
            state,
            server_version,
        } = &mut *self.state.write().unwrap_or_log()
        else {
            panic!("App::display_gui_connected must only be called when state == AppState::Connected, not {:?}", self.state.read().unwrap_or_log());
        };

        egui::CentralPanel::default().show(ctx, |ui| {
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
                        Some(state.effect_name)
                    } else {
                        None
                    }
                })
                .inner
                .flatten();

            let restart_effect = ui.button("Restart current effect").clicked();

            let effect_config_changed = if let Some(config) = &mut state.effect_config {
                ui.separator();
                config.render_options_gui(ctx, ui).then_some(config.clone())
            } else {
                None
            };

            // TODO: Collapse these cases into a single `self.async_runtime.spawn_pinned()`
            // call to reduce overhead
            if let Some(name) = new_effect_selected {
                trace!("New effect selected, sending message");

                self.async_runtime.spawn_pinned({
                    let message_tx = self.message_tx.clone();

                    move || async move {
                        message_tx
                            .send(ClientToServerMsg::ChangeEffect(name))
                            .await
                            .expect_or_log("Unable to send ChangeEffect message down channel");
                    }
                });
            }

            if restart_effect {
                trace!("Requesting to restart current effect, sending message");

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
                trace!("Effect config changed, sending message");

                self.async_runtime.spawn_pinned({
                    let message_tx = self.message_tx.clone();

                    move || async move {
                        message_tx
                            .send(ClientToServerMsg::UpdateConfig(config))
                            .await
                            .expect_or_log("Unable to send UpdateConfig message down channel");
                    }
                });
            }
        });

        egui::TopBottomPanel::bottom("about-panel").show(ctx, |ui| {
            ui.heading("Winter WonderLights");

            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 0.;
                ui.label("Winter WonderLights is free, open-source software. It is available on ");
                ui.hyperlink_to("GitHub", "https://github.com/DoctorDalek1963/winter-wonderlights/");
                ui.label(".");
            });

            ui.add_space(12.);
            ui.label("We accept pull requests, so if you want to add a new effect, add a new driver, improve performance, \
                improve UI/UX, or improve the project in any other way, then feel free to open a PR!");
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 0.;
                ui.label("Even if you're not a programmer, anyone can ");
                ui.hyperlink_to("open an issue", "https://github.com/DoctorDalek1963/winter-wonderlights/issues/new");
                ui.label(" to suggest any additions or improvements, or to report any bugs.");
            });

            ui.add_space(12.);
            ui.heading("Version numbers");
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 10.;
                ui.label(format!("ww-client: v{}", crate::CRATE_VERSION));
                ui.label(format!("ww-shared: v{}", ww_shared::CRATE_VERSION));
                ui.label(format!("ww-server: v{server_version}"));
            });
            ui.add_space(6.);
        });
    }

    /// Display the GUI for a protocol mismatch. This method assumes that [`Self::state`] is
    /// [`AppState::ProtocolMismatch`] and will panic if it's not.
    #[instrument(skip_all)]
    fn display_gui_protocol_mismatch(&mut self, ctx: &eframe::egui::Context) {
        let AppState::ProtocolMismatch {
            server_version,
            server_protocol_version,
        } = &*self.state.read().unwrap_or_log()
        else {
            panic!("App::display_gui_protocol_mismatch must only be called when state == AppState::ProtocolMismatch, not {:?}", self.state.read());
        };

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered_justified(|ui| {
                ui.heading(RichText::new("Protocol mismatch!").heading().strong());
            });

            ui.add_space(12.);
            ui.label(format!(
                "This website is using the Winter WonderLights client version {} and protocol version {}",
                crate::CRATE_VERSION,
                ww_shared::CRATE_VERSION
            ));
            ui.label(format!("However, the server is using Winter WonderLights server version {server_version} \
                    and protcol version {server_protocol_version}"));

            ui.add_space(12.);
            ui.label("Since the protocol versions are different, we cannot communicate with the server.");

            ui.add_space(12.);
            ui.label("To fix this problem, reload the page and if the problem persists, contact the person who \
                set up Winter WonderLights for your tree.");
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 0.;
                ui.label("If you are the person who set this up, try recompiling everything from the most recent tagged commit. See ");
                ui.hyperlink_to("the GitHub",  "https://github.com/DoctorDalek1963/winter-wonderlights/");
                ui.label(".");
            });
        });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        self.respond_to_server_messages();

        let state = self.state.read().unwrap_or_log().clone();
        match state {
            AppState::WaitingForConnection => self.display_gui_waiting_for_connection(ctx),
            AppState::Connected { .. } => self.display_gui_connected(ctx),
            AppState::ProtocolMismatch { .. } => self.display_gui_protocol_mismatch(ctx),
        };

        // We need to constantly be repainting the GUI so that new server messages are always
        // processed
        ctx.request_repaint();
    }
}
