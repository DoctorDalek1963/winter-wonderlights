//! This binary crate just runs the server for Winter WonderLights, currently just to test
//! features.

mod drivers;

use color_eyre::Result;
use lazy_static::lazy_static;
use std::{
    ops::Deref,
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};
use tiny_http::{Header, Request, Response};
use tokio::sync::broadcast;
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
        info!("Saving config to file");

        if let Some(config) = &self
            .read()
            .expect_or_log("Should be able to read client state")
            .effect_config
        {
            config.save_to_file(&get_config_filename(config.effect_name()));
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

/// Create a header that will allow the client to function properly without CORS getting in the way.
fn no_cors_header() -> Header {
    Header {
        field: "Access-Control-Allow-Origin"
            .parse()
            .expect_or_log("This &'static str should parse just fine"),
        value: "*"
            .parse()
            .expect_or_log("This &'static str should parse just fine"),
    }
}

#[instrument(skip_all, fields(addr = ?req.remote_addr()))]
fn handle_request(mut req: Request, client_state: &WrappedClientState) -> Result<()> {
    trace!(?req, "Received a new request");

    let mut body = String::new();
    req.as_reader().read_to_string(&mut body)?;
    let msg: ClientToServerMsg = ron::from_str(&body)?;

    trace!(?msg);

    /// Lock the local `client_state` for writing.
    ///
    /// NOTE: If you assign this to a variable, remember to drop it before calling
    /// [`respond_update_client_state`], or else a deadlock will occur.
    macro_rules! write_state {
        () => {
            client_state
                .write()
                .expect_or_log("Should be able to write to client state")
        };
    }

    /// Lock the local `client_state` for reading.
    macro_rules! read_state {
        () => {
            client_state
                .read()
                .expect_or_log("Should be able to read client state")
        };
    }

    /// Send an `UpdateClientState` response.
    macro_rules! respond_update_client_state {
        () => {
            req.respond(
                Response::from_string(
                    ron::to_string(&ServerToClientMsg::UpdateClientState(read_state!().clone()))
                        .expect_or_log(EXPECT_SERIALIZE_MSG),
                )
                .with_header(no_cors_header()),
            )?
        };
    }

    match msg {
        ClientToServerMsg::RequestUpdate => {
            // This is debug rather than info because the client does it every second
            debug!("Client requesting update");

            respond_update_client_state!();
        }
        ClientToServerMsg::UpdateConfig(new_config) => {
            info!(?new_config, "Client requesting config change");

            let mut client = write_state!();
            client.effect_config = Some(new_config);

            trace!(?client, "After updating client state config");
            drop(client);
            respond_update_client_state!();
        }
        ClientToServerMsg::ChangeEffect(new_effect) => {
            info!(?new_effect, "Client requesting effect change");

            client_state.save_config();

            let mut client = write_state!();
            client.effect_name = new_effect;
            client.effect_config = new_effect.map(|effect| effect.config_from_file());

            debug!(?client, "After updating client state effect name");
            drop(client);

            SEND_MESSAGE_TO_THREAD
                .send(ThreadMessage::Restart)
                .expect_or_log("Unable to send ThreadMessage::Restart");

            respond_update_client_state!();
        }
        ClientToServerMsg::RestartCurrentEffect => {
            info!("Client requesting restart current effect");

            SEND_MESSAGE_TO_THREAD
                .send(ThreadMessage::Restart)
                .expect_or_log("Unable to send ThreadMessage::Restart");

            respond_update_client_state!();
        }
    };

    Ok(())
}

/// Run the effect in the `state` with `tokio` and listen for messages on the
/// [`struct@SEND_MESSAGE_TO_THREAD`] channel. Intended to be run in a background thread.
#[instrument(skip_all)]
fn run_effect(state: WrappedClientState) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap_or_log();
    let local = tokio::task::LocalSet::new();

    let mut driver = self::drivers::Driver::init();
    let mut rx = SEND_MESSAGE_TO_THREAD.subscribe();

    info!("Beginning tokio listen and run loop");

    /// Lock the local `state` for reading.
    macro_rules! read_state {
        () => {
            state
                .read()
                .expect_or_log("Should be able to read client state")
        };
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
                                read_state!()
                                    .effect_name
                                    .map_or("None", |x| x.effect_name())
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
                    let locked_state = read_state!();
                    let effect_name = locked_state.effect_name;
                    drop(locked_state);

                    if let Some(effect) = effect_name {
                        let effect: EffectDispatchList = effect.into();

                        effect.run(&mut driver).await;
                        driver.display_frame(FrameType::Off);

                        // Pause before looping the effect
                        // TODO: Allow custom pause time
                        tokio::time::sleep(Duration::from_millis(500)).await;

                        info!("Looping effect");
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

#[instrument]
fn main() {
    init_tracing();

    info!(port = env!("PORT"), "Initialising server");

    let server = match tiny_http::Server::https(
        concat!("localhost:", env!("PORT")),
        tiny_http::SslConfig {
            certificate: include_bytes!(env!("SERVER_SSL_CERT_PATH")).to_vec(),
            private_key: include_bytes!(env!("SERVER_SSL_KEY_PATH")).to_vec(),
        },
    ) {
        Ok(server) => server,
        Err(error) => {
            warn!(
                ?error,
                "Error creating HTTPS server; defaulting to HTTP server"
            );
            tiny_http::Server::http(concat!("localhost:", env!("PORT")))
                .expect_or_log("Unable to create HTTP server")
        }
    };

    let client_state = WrappedClientState::new();

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

    for req in server.incoming_requests() {
        match handle_request(req, &client_state) {
            Ok(()) => (),
            Err(e) => error!(?e, "Error handing request"),
        };
    }

    info!("Server socket has shut down. Saving config and terminating server");

    client_state.save_config();
    info!("Config saved");
}
