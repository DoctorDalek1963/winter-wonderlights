//! This module handles running the actual server.

use crate::{
    run_effect::{ThreadMessage, SEND_MESSAGE_TO_RUN_EFFECT_THREAD},
    WrappedClientState,
};
use color_eyre::{Report, Result};
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt};
use lazy_static::lazy_static;
use rustls_pemfile::{certs, pkcs8_private_keys};
use std::{
    fs::File,
    io::{self, BufReader},
    net::SocketAddr,
    path::Path,
    sync::Arc,
    thread,
    time::Duration,
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{broadcast, oneshot},
};
use tokio_rustls::{
    rustls::{self, Certificate, PrivateKey},
    TlsAcceptor,
};
use tokio_stream::wrappers::BroadcastStream;
use tokio_tungstenite::tungstenite;
use tracing::{debug, error, info, instrument, trace, warn};
use tracing_unwrap::ResultExt;
use ww_shared::{ClientToServerMsg, ServerToClientMsg};

lazy_static! {
    /// The broadcast sender which lets you send messages between client tasks to broadcast a
    /// message to all connected clients.
    static ref SEND_MESSAGE_BETWEEN_CLIENT_TASKS: broadcast::Sender<Vec<u8>> = broadcast::channel(100).0;
}

/// Terminate all the current client connections.
#[instrument]
pub fn terminate_all_client_connections() {
    info!("Terminating all client connections");
    SEND_MESSAGE_BETWEEN_CLIENT_TASKS
        .send(
            bincode::serialize(&ServerToClientMsg::TerminateConnection)
                .expect_or_log("Serializing a ServerToClientMsg should never fail"),
        )
        .expect_or_log("Should be able to send message down SEND_MESSAGE_BETWEEN_CLIENT_TASKS");
}

/// Handle a single connection.
#[instrument(skip_all, fields(?addr))]
async fn handle_connection(
    tls_acceptor: TlsAcceptor,
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

    let socket = tls_acceptor
        .accept(socket)
        .await
        .expect_or_log("Should be able to accept TLS connection");

    let ws_stream = tokio_tungstenite::accept_async(socket)
        .await
        .expect_or_log("Error during WebSocket handshake");

    info!("Created a new connection");

    let (outgoing, incoming) = ws_stream.split();

    // Read messages from this client and broadcast them down [`SEND_MESSAGE_BETWEEN_CLIENT_TASKS`].
    let broadcast_incoming = incoming.try_for_each(|msg| {
        let send_message = |message: &ServerToClientMsg| {
            SEND_MESSAGE_BETWEEN_CLIENT_TASKS
                .send(
                    bincode::serialize(message)
                        .expect_or_log("Serializing a ServerToClientMsg should never fail"),
                )
                .expect_or_log(
                    "Should be able to send message down SEND_MESSAGE_BETWEEN_CLIENT_TASKS",
                )
        };

        let send_update_client_state = || {
            send_message(&ServerToClientMsg::UpdateClientState(
                client_state
                    .read()
                    .expect_or_log("Should be able to read client state")
                    .clone(),
            ))
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
            ClientToServerMsg::EstablishConnection { protocol_version } => {
                info!(
                    client_protocol_version = ?protocol_version,
                    "Establishing connection with client"
                );

                if protocol_version == ww_shared::CRATE_VERSION {
                    debug!("Protocol versions match, accepting client");

                    send_message(&ServerToClientMsg::EstablishConnection {
                        protocol_version: ww_shared::CRATE_VERSION.to_string(),
                        server_version: crate::CRATE_VERSION.to_string(),
                    });
                } else {
                    warn!(
                        client_protocol_version = protocol_version,
                        server_protocol_version = ww_shared::CRATE_VERSION,
                        "Protocol version mismatch, denying client"
                    );

                    send_message(&ServerToClientMsg::DenyConnection {
                        protocol_version: ww_shared::CRATE_VERSION.to_string(),
                        server_version: crate::CRATE_VERSION.to_string(),
                    });
                }
            }
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

                SEND_MESSAGE_TO_RUN_EFFECT_THREAD
                    .send(ThreadMessage::Restart)
                    .expect_or_log("Unable to send ThreadMessage::Restart");

                send_update_client_state();
            }
            ClientToServerMsg::RestartCurrentEffect => {
                info!("Client requesting restart current effect");

                SEND_MESSAGE_TO_RUN_EFFECT_THREAD
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

/// Read the file at the given path and try to read it as a list of SSL certificates.
fn load_certs(path: &Path) -> io::Result<Vec<Certificate>> {
    certs(&mut BufReader::new(File::open(path)?))
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid certificate"))
        .map(|mut certs| certs.drain(..).map(Certificate).collect())
}

/// Read the file at the given path and try to read it as a list of SSL private keys.
fn load_keys(path: &Path) -> io::Result<Vec<PrivateKey>> {
    pkcs8_private_keys(&mut BufReader::new(File::open(path)?))
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid key"))
        .map(|mut keys| keys.drain(..).map(PrivateKey).collect())
}

/// Run the server asynchronously.
pub async fn run_server(
    client_state: WrappedClientState,
    kill_run_effect_thread: oneshot::Receiver<()>,
) -> Result<()> {
    info!(port = env!("PORT"), "Initialising server");

    let certs = load_certs(&Path::new(env!("SERVER_SSL_CERT_PATH")))
        .expect_or_log("Unable to load SSL certificates");
    let mut keys =
        load_keys(&Path::new(env!("SERVER_SSL_KEY_PATH"))).expect_or_log("Unable to load SSL keys");
    assert!(certs.len() > 0, "We need to have at least one certificate");
    assert!(keys.len() > 0, "We need to have at least one key");

    debug!(?certs, ?keys);

    let tls_config = rustls::ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs, keys.remove(0))
        .expect_or_log("Unable to create rustls config");
    let tls_acceptor = TlsAcceptor::from(Arc::new(tls_config));

    let listener = TcpListener::bind(concat!("0.0.0.0:", env!("PORT")))
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

    let run_effect_thread = thread::Builder::new()
        .name("run-effect".to_string())
        .spawn({
            let state = client_state.clone();
            move || crate::run_effect::run_effect(state, kill_run_effect_thread)
        })
        .unwrap_or_log();

    info!("Server initialised");

    let accept_new_connections = async move {
        while let Ok((socket, addr)) = listener.accept().await {
            let tls_acceptor = tls_acceptor.clone();
            let client_state = client_state.clone();

            tokio::spawn(async move {
                match handle_connection(tls_acceptor, socket, addr, client_state).await {
                    Ok(_) => (),
                    Err(e) => error!(?e, "Error handling connection"),
                }
            });
        }
    };

    tokio::select! {
        biased;

        // If the run-effect thread is finished, then the driver has stopped for whatever reason,
        // so kill the server
        _ = async move {
            while let false = run_effect_thread.is_finished() {
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        } => {
            error!("run-effect thread has terminated prematurely. Killing server");
            return Err(Report::msg("run-effect terminated prematurely"));
        }

        _ = accept_new_connections => {}
    }

    Ok(())
}
