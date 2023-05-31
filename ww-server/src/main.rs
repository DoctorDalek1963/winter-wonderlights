//! This binary crate just runs the server for Winter WonderLights, currently just to test
//! features.

mod drivers;

use color_eyre::Result;
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt};
use lazy_static::lazy_static;
use std::{
    io,
    net::SocketAddr,
    ops::Deref,
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};
use tokio::{
    net::{TcpListener, TcpStream},
    signal,
    sync::broadcast,
};
use tokio_stream::wrappers::BroadcastStream;
use tokio_tungstenite::tungstenite;
use tracing::{debug, error, info, instrument, trace, warn};
use tracing_subscriber::{filter::LevelFilter, fmt::Layer, prelude::*, EnvFilter};
use tracing_unwrap::ResultExt;
use ww_driver_trait::Driver;
use ww_effects::{traits::get_config_filename, EffectDispatchList};
use ww_frame::FrameType;
use ww_shared::{ClientState, ClientToServerMsg, ServerToClientMsg};

/// The `.expect()` error message for serializing a [`ServerToClientMsg`].
const EXPECT_SERIALIZE_MSG: &str = "Serializing a ServerToClientMsg should never fail";

/// The filename for the server state config.
const SERVER_STATE_FILENAME: &str = "server_state.ron";

lazy_static! {
    /// The broadcast sender which lets you send messages to the background thread, which is
    /// running the effect itself.
    static ref SEND_MESSAGE_TO_THREAD: broadcast::Sender<ThreadMessage> = broadcast::channel(10).0;

    /// The broadcast sender which lets you send messages between client tasks to broadcast a
    /// message to all connected clients.
    static ref SEND_MESSAGE_BETWEEN_CLIENT_TASKS: broadcast::Sender<Vec<u8>> = broadcast::channel(100).0;
}

/// Possible messages to send to the effect thread.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ThreadMessage {
    /// Restart the effect rendering.
    ///
    /// This could be because a new effect was selected or because the client wanted to restart the
    /// current effect.
    Restart,
}

/// A simple wrapper struct to hold the client state.
#[derive(Clone, Debug)]
struct WrappedClientState(Arc<RwLock<ClientState>>);

impl WrappedClientState {
    /// Initialise the server state.
    fn new() -> Self {
        Self(Arc::new(RwLock::new(ClientState::from_file(
            SERVER_STATE_FILENAME,
        ))))
    }

    /// Save the config of the client state.
    #[instrument(skip_all)]
    fn save_config(&self) {
        if let Some(config) = &self
            .read()
            .expect_or_log("Should be able to read client state")
            .effect_config
        {
            info!(?config, "Saving config to file");
            config.save_to_file(&get_config_filename(config.effect_name()));
        } else {
            debug!("Tried to save config but it's None, so skipping");
        }
    }
}

impl Deref for WrappedClientState {
    type Target = RwLock<ClientState>;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl Drop for WrappedClientState {
    fn drop(&mut self) {
        self.read()
            .unwrap_or_log()
            .save_to_file(SERVER_STATE_FILENAME)
    }
}

/// Handle a single connection.
#[instrument(skip_all, fields(?addr))]
async fn handle_connection(
    socket: TcpStream,
    addr: SocketAddr,
    client_state: WrappedClientState,
) -> Result<()> {
    /// Lock the local `client_state` for writing.
    macro_rules! write_state {
        ($name:ident => $body:expr) => {{
            let mut $name = client_state
                .write()
                .expect_or_log("Should be able to write to client state");
            $body;
            drop($name);
        }};
    }

    let ws_stream = tokio_tungstenite::accept_async(socket)
        .await
        .expect_or_log("Error during WebSocket handshake");

    info!("Created a new connection");

    let (outgoing, incoming) = ws_stream.split();

    // Read messages from this client and broadcast them down
    // [`SEND_MESSAGE_BETWEEN_CLIENT_TASKS`].
    let broadcast_incoming = incoming.try_for_each(|msg| {
        let send_update_client_state = || {
            SEND_MESSAGE_BETWEEN_CLIENT_TASKS
                .send(
                    bincode::serialize(&ServerToClientMsg::UpdateClientState(
                        client_state
                            .read()
                            .expect_or_log("Should be able to read client state")
                            .clone(),
                    ))
                    .expect_or_log(EXPECT_SERIALIZE_MSG),
                )
                .expect_or_log("Should be able to send message down tx")
        };

        let bytes = match msg {
            tungstenite::Message::Binary(bytes) => bytes,
            _ => {
                return future::err(tungstenite::Error::Protocol(
                    tungstenite::error::ProtocolError::ExpectedFragment(
                        tungstenite::protocol::frame::coding::Data::Binary,
                    ),
                ))
            }
        };

        let msg: ClientToServerMsg = match bincode::deserialize(&bytes) {
            Ok(x) => x,
            Err(e) => {
                error!(?e, "Unable to deserialize client message; disconnecting it");
                return future::err(tungstenite::Error::Io(io::Error::new(
                    io::ErrorKind::InvalidData,
                    e,
                )));
            }
        };

        match msg {
            ClientToServerMsg::RequestUpdate => {
                info!("Client requesting update");

                send_update_client_state();
            }
            ClientToServerMsg::UpdateConfig(new_config) => {
                info!(?new_config, "Client requesting config change");

                write_state!(state => {
                    state.effect_config = Some(new_config);
                    trace!(?state, "After updating client state config");
                });
                send_update_client_state();
            }
            ClientToServerMsg::ChangeEffect(new_effect) => {
                info!(?new_effect, "Client requesting effect change");

                client_state.save_config();

                write_state!(state => {
                    state.effect_name = new_effect;
                    state.effect_config = new_effect.map(|effect| effect.config_from_file());
                    debug!(?state, "After updating client state effect name");
                });

                SEND_MESSAGE_TO_THREAD
                    .send(ThreadMessage::Restart)
                    .expect_or_log("Unable to send ThreadMessage::Restart");

                send_update_client_state();
            }
            ClientToServerMsg::RestartCurrentEffect => {
                info!("Client requesting restart current effect");

                SEND_MESSAGE_TO_THREAD
                    .send(ThreadMessage::Restart)
                    .expect_or_log("Unable to send ThreadMessage::Restart");

                send_update_client_state();
            }
        };

        future::ok(())
    });

    // Receive messages from other client connections via [`SEND_MESSAGE_BETWEEN_CLIENT_TASKS`] and
    // forward these messages to this client through the WS outgoing half.
    let receive_from_other_clients = {
        let rx = BroadcastStream::new(SEND_MESSAGE_BETWEEN_CLIENT_TASKS.subscribe());
        rx.map(|bytes| {
            Ok(tungstenite::Message::Binary(
                bytes.expect_or_log("Error in receiving message down channel"),
            ))
        })
        .forward(outgoing)
    };

    pin_mut!(broadcast_incoming, receive_from_other_clients);
    future::select(broadcast_incoming, receive_from_other_clients).await;

    info!("Disconnecting client");

    Ok(())
}

/// Run the effect in the `state` with `tokio` and listen for messages on the
/// [`struct@SEND_MESSAGE_TO_THREAD`] channel. Intended to be run in a background thread.
#[instrument(skip_all)]
fn run_effect(client_state: WrappedClientState) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap_or_log();
    let local = tokio::task::LocalSet::new();

    let mut driver = self::drivers::Driver::init();
    let mut rx = SEND_MESSAGE_TO_THREAD.subscribe();

    info!("Beginning tokio listen and run loop");

    /// Lock the local `client_state` for reading.
    macro_rules! read_state {
        ($name:ident => $body:expr) => {{
            let $name = client_state
                .read()
                .expect_or_log("Should be able to write to client state");
            let rv = $body;
            drop($name);
            rv
        }};
    }

    local.block_on(&runtime, async move {
        loop {
            tokio::select! {
                biased;

                // First, we check if we've received a message on the channel and respond to it if so
                msg = rx.recv() => {
                    trace!(?msg, "Received ThreadMessage");

                    match msg.expect_or_log("There should not be an error in receiving from the channel") {
                        ThreadMessage::Restart => {
                            info!(
                                "Restarting effect {:?}",
                                read_state!(state => state.effect_name.map_or("None", |x| x.effect_name()))
                            );
                            continue;
                        }
                    };
                }

                // Then, we run the effect in a loop. Most of the effect time is awaiting
                // sleeps, and control gets yielded back to `select!` while that's happening,
                // so it can respond to messages quickly
                _ = async { loop {
                    // We have to get the effect and then drop the lock so that the
                    // `handle_request()` function can actually write to the client state when the
                    // client requests an effect change
                    let effect_name = read_state!(state => state.effect_name);

                    if let Some(effect) = effect_name {
                        let effect: EffectDispatchList = effect.into();

                        effect.run(&mut driver).await;
                        driver.display_frame(FrameType::Off);

                        // Pause before looping the effect
                        // TODO: Allow custom pause time
                        tokio::time::sleep(Duration::from_millis(500)).await;

                        info!("Looping effect {:?}", read_state!(state => state.effect_name.map_or("None", |x| x.effect_name())));
                    } else {
                        driver.display_frame(FrameType::Off);

                        // Don't send `FrameType::Off` constantly. `select!` takes control
                        // while we're awaiting anyway, so responding to a message will be fast
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }} => {}
            }
        }
    });
}

/// Initialise a subscriber for tracing to log to `stdout` and a file.
fn init_tracing() {
    let appender =
        tracing_appender::rolling::daily(concat!(env!("DATA_DIR"), "/logs"), "server.log");

    let subscriber = tracing_subscriber::registry()
        .with(
            Layer::new()
                .with_writer(appender)
                .with_ansi(false)
                .with_filter(
                    EnvFilter::builder()
                        .with_default_directive(LevelFilter::DEBUG.into())
                        .parse_lossy(""),
                ),
        )
        .with(
            Layer::new()
                .with_writer(std::io::stdout)
                .with_ansi(true)
                .with_filter(EnvFilter::from_default_env().add_directive(LevelFilter::INFO.into())),
        );

    tracing::subscriber::set_global_default(subscriber)
        .expect_or_log("Setting the global default for tracing should be okay");
}

/// Run the server asynchronously.
async fn run_server(client_state: WrappedClientState) {
    info!(port = env!("PORT"), "Initialising server");

    let listener = TcpListener::bind(concat!("localhost:", env!("PORT")))
        .await
        .expect_or_log("Unable to start TcpListener");

    // Save the effect config every 10 seconds
    thread::Builder::new()
        .name("client-state-save-config".to_string())
        .spawn({
            let state = client_state.clone();
            move || loop {
                state.save_config();
                thread::sleep(Duration::from_secs(10));
            }
        })
        .unwrap_or_log();

    thread::Builder::new()
        .name("run-effect".to_string())
        .spawn({
            let state = client_state.clone();
            move || run_effect(state)
        })
        .unwrap_or_log();

    info!("Server initialised");

    while let Ok((socket, addr)) = listener.accept().await {
        let client_state = client_state.clone();
        tokio::spawn(async move {
            match handle_connection(socket, addr, client_state).await {
                Ok(_) => (),
                Err(e) => error!(?e, "Error handling connection"),
            }
        });
    }
}

#[tokio::main]
#[instrument]
async fn main() {
    init_tracing();

    let client_state = WrappedClientState::new();

    tokio::select! {
        biased;

        _ = signal::ctrl_c() => {
            info!("Recieved SIGINT. Terminating");
            client_state.save_config();
            return;
        }
        _ = run_server(client_state.clone()) => {}
    }
}
