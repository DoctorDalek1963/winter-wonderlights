//! This module handles the [`App`] type for the `eframe`-based GUI.

use crossbeam_channel::{Receiver, Sender};
use eframe::egui::{self, Context};
use reqwest::Client;
use std::time::Duration;
use tracing::{debug, error, instrument};
use tracing_unwrap::ResultExt;
use ww_shared_msgs::{ClientToServerMsg, ServerToClientMsg};

/// The `.expect()` error message for serializing a [`ClientToServerMsg`].
const EXPECT_SERIALIZE_MSG: &str = "Serializing a ClientToServerMsg should never fail";

/// The app type itself.
pub struct App {
    /// The receiver end of a channel used to receive messages from the server.
    server_rx: Receiver<ServerToClientMsg>,

    /// TODO: Remove this
    demo_number: u32,
}

impl App {
    /// Create a new [`App`] and initialise sound background processes.
    pub fn new(_cc: &eframe::CreationContext) -> Self {
        let (tx, server_rx) = crossbeam_channel::unbounded();
        let client = Client::new();

        let app = Self {
            server_rx,
            demo_number: 0,
        };

        prokio::Runtime::default().spawn_pinned(move || Self::request_updates(client, tx));

        app
    }

    /// Send a [`RequestUpdate`](ClientToServerMsg::RequestUpdate) message once every second.
    #[instrument(skip_all)]
    async fn request_updates(client: Client, tx: Sender<ServerToClientMsg>) {
        loop {
            match client
                .post(env!("SERVER_URL"))
                .body(
                    ron::to_string(&ClientToServerMsg::RequestUpdate)
                        .expect_or_log(EXPECT_SERIALIZE_MSG),
                )
                .send()
                .await
            {
                Ok(response) => match response.text().await {
                    Ok(body) => match ron::from_str(&body) {
                        Ok(msg) => match tx.send(msg) {
                            Ok(()) => (),
                            Err(e) => error!(?e, "Error sending message down channel"),
                        },
                        Err(e) => error!(?e, "Error deserializing message from server"),
                    },
                    Err(e) => error!(?e, "Error getting text from response"),
                },
                Err(e) => error!(?e, "Error communicating with server"),
            }
            prokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    /// Respond to all the server messages on `self.server_rx`.
    #[instrument(skip_all)]
    fn respond_to_server_messages(&mut self) {
        while let Ok(msg) = self.server_rx.try_recv() {
            debug!(?msg, "Responding to server message");

            match msg {
                ServerToClientMsg::UpdateClientState => self.demo_number += 1,
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.respond_to_server_messages();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Hello!!!");
            ui.label(format!("{}", self.demo_number));
        });

        // We need to constantly be repainting the GUI so that new server messages are always
        // processed.
        ctx.request_repaint();
    }
}
