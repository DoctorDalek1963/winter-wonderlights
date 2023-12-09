//! This module handles running the actual server.

use crate::{
    scan_manager::{ScanManagerMsg, SEND_MESSAGE_TO_SCAN_MANAGER},
    state::{Connection, ConnectionSocket, ScannerState},
};
use color_eyre::{Report, Result};
use futures_util::{future, pin_mut, stream::TryStreamExt, SinkExt, StreamExt};
use lazy_static::lazy_static;
use std::{
    io,
    net::SocketAddr,
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};
use tokio::{
    net::TcpListener,
    sync::{oneshot, watch},
};
use tokio_stream::wrappers::WatchStream;
use tokio_tungstenite::tungstenite;
use tracing::{debug, error, info, instrument, warn};
use tracing_unwrap::ResultExt;
use ww_scanner_shared::{
    CameraToServerMsg, ClientType, ControllerToServerMsg, GenericServerToClientMsg,
    ServerToCameraMsg, ServerToControllerMsg, DECLARE_CLIENT_TYPE_MAGIC,
};

lazy_static! {
    /// Send messages to the camera client.
    pub static ref CAMERA_SEND: watch::Sender<Vec<u8>> = watch::channel(vec![]).0;

    /// Send messages to the controller client.
    pub static ref CONTROLLER_SEND: watch::Sender<Vec<u8>> = watch::channel(vec![]).0;

    /// The global state of the scanner.
    pub static ref SCANNER_STATE: Arc<RwLock<ScannerState>> = Arc::new(RwLock::new(ScannerState::new()));
}

/// Read from [`struct@SCANNER_STATE`].
macro_rules! scanner_state_read {
    () => {
        SCANNER_STATE
            .read()
            .expect_or_log("Should be able to read from SCANNER_STATE")
    };
}

/// Write to [`struct@SCANNER_STATE`]
macro_rules! scanner_state_write {
    ($name:ident => $body:expr) => {{
        let mut $name = SCANNER_STATE
            .write()
            .expect_or_log("Should be able to write to SCANNER_STATE");
        $body;
        drop($name);
    }};
}

/// Terminate all the current client connections.
#[instrument]
pub fn terminate_all_client_connections() {
    info!("Terminating all client connections");
    let _ = CAMERA_SEND.send(
        bincode::serialize(&ServerToCameraMsg::Generic(
            GenericServerToClientMsg::TerminateConnection,
        ))
        .expect_or_log("Serializing a ServerToCameraMsg should never fail"),
    );
    let _ = CONTROLLER_SEND.send(
        bincode::serialize(&ServerToControllerMsg::Generic(
            GenericServerToClientMsg::TerminateConnection,
        ))
        .expect_or_log("Serializing a ServerToControllerMsg should never fail"),
    );
}

/// Handle a single connection from a client.
#[instrument(skip_all, fields(?addr))]
async fn handle_connection(socket: ConnectionSocket, addr: SocketAddr) -> Result<()> {
    let ws_stream = tokio_tungstenite::accept_async(socket)
        .await
        .expect_or_log("Error during WebSocket handshake");

    info!("Created a new connection");

    let (outgoing, mut incoming) = ws_stream.split();

    let first_msg = incoming.try_next().await?;
    let Some(tungstenite::Message::Binary(bytes)) = first_msg else {
        debug!(
            ?first_msg,
            "First message from client not Some(Binary(...))"
        );
        return Err(tungstenite::Error::Protocol(
            tungstenite::error::ProtocolError::ExpectedFragment(
                tungstenite::protocol::frame::coding::Data::Binary,
            ),
        )
        .into());
    };

    debug!(?bytes, "First message from new client");

    let client_type: ClientType = if bytes.len() == 4 && bytes[..3] == DECLARE_CLIENT_TYPE_MAGIC {
        bytes[3]
            .try_into()
            .map_err(|text| Report::msg(format!("{text} (byte={})", bytes[3])))?
    } else {
        error!(?bytes, "Expected ClientType declaration as first message");
        return Err(Report::msg(
            "Expected ClientType declaration as first message",
        ));
    };

    let conn = Connection {
        addr,
        incoming,
        outgoing,
    };

    match client_type {
        ClientType::Camera => {
            let ret = handle_camera_connection(conn).await;
            scanner_state_write!(state => state.disconnect_camera());
            ret?;
        }
        ClientType::Controller => {
            let ret = handle_controller_connection(conn).await;
            scanner_state_write!(state => state.disconnect_controller());
            ret?;
        }
    }

    Ok(())
}

/// Handle a new connection from a camera client.
async fn handle_camera_connection(mut conn: Connection) -> Result<()> {
    if scanner_state_read!().camera_conn {
        info!("Rejecting camera client");
        conn.outgoing
            .send(tungstenite::Message::Binary(
                bincode::serialize(&ServerToCameraMsg::Generic(
                    GenericServerToClientMsg::RejectConnection,
                ))
                .expect_or_log("Should be able to serialize RejectConnection"),
            ))
            .await
            .expect_or_log("Should be able to send message to client");
    }

    info!("Connecting camera client");
    scanner_state_write!(state => state.camera_conn = true);

    // Receive messages from the client and act on them
    let incoming_msgs = conn.incoming.try_for_each(|msg| {
        let tungstenite::Message::Binary(bytes) = msg else {
            return future::err(tungstenite::Error::Protocol(
                tungstenite::error::ProtocolError::ExpectedFragment(
                    tungstenite::protocol::frame::coding::Data::Binary,
                ),
            ));
        };

        let msg: CameraToServerMsg = match bincode::deserialize(&bytes) {
            Ok(x) => x,
            Err(e) => {
                error!(
                    ?e,
                    "Unable to deserialize camera client message; disconnecting it"
                );
                return future::err(tungstenite::Error::Io(io::Error::new(
                    io::ErrorKind::InvalidData,
                    e,
                )));
            }
        };

        debug!(?msg, "Received and deserialized message from camera client");

        match msg {
            CameraToServerMsg::DeclareClientType => {
                error!("Recieved CameraToServerMsg::DeclareClientType after handshake");
                return future::err(tungstenite::Error::Protocol(
                    tungstenite::error::ProtocolError::ExpectedFragment(
                        tungstenite::protocol::frame::coding::Data::Binary,
                    ),
                ));
            }
            CameraToServerMsg::EstablishConnection(camera_info) => {
                debug!("Camera trying to establish connection; sending AcceptConnection");
                scanner_state_write!(state => state.camera_info = Some(camera_info));
                CAMERA_SEND
                    .send(
                        bincode::serialize(&ServerToCameraMsg::Generic(
                            GenericServerToClientMsg::AcceptConnection,
                        ))
                        .expect_or_log("Should be able to serialize AcceptConnection"),
                    )
                    .expect_or_log("Should be able to send message down CAMERA_SEND");
            }
            CameraToServerMsg::PhotoTaken {
                light_idx,
                brightest_pixel_pos,
            } => {
                debug!(
                    ?light_idx,
                    ?brightest_pixel_pos,
                    "Received photo data from the camera"
                );
                SEND_MESSAGE_TO_SCAN_MANAGER
                    .send(ScanManagerMsg::ReceivedPhoto {
                        light_idx,
                        brightest_pixel_pos,
                    })
                    .expect_or_log(
                        "Should be able to send message down SEND_MESSAGE_TO_SCAN_MANAGER",
                    );
            }
        }

        future::ok(())
    });

    // Forward messages from CAMERA_SEND to the actual connection
    let recv_from_server = WatchStream::new(CAMERA_SEND.subscribe())
        .filter_map(|bytes| {
            future::ready(if !bytes.is_empty() {
                debug!(?bytes, "Forwarding message to camera");
                Some(Ok(tungstenite::Message::Binary(bytes)))
            } else {
                None
            })
        })
        .forward(conn.outgoing);

    SEND_MESSAGE_TO_SCAN_MANAGER
        .send(ScanManagerMsg::CameraConnected)
        .expect_or_log("Should be able to send message down SEND_MESSAGE_TO_SCAN_MANAGER");

    pin_mut!(incoming_msgs, recv_from_server);
    future::select(incoming_msgs, recv_from_server).await;

    info!("Disconnecting camera");
    SEND_MESSAGE_TO_SCAN_MANAGER
        .send(ScanManagerMsg::CameraDisconnected)
        .expect_or_log("Should be able to send message down SEND_MESSAGE_TO_SCAN_MANAGER");

    Ok(())
}

/// Handle a new connection from a controller client.
async fn handle_controller_connection(mut conn: Connection) -> Result<()> {
    if scanner_state_read!().controller_conn {
        info!("Rejecting controller client");
        conn.outgoing
            .send(tungstenite::Message::Binary(
                bincode::serialize(&ServerToCameraMsg::Generic(
                    GenericServerToClientMsg::RejectConnection,
                ))
                .expect_or_log("Should be able to serialize RejectConnection"),
            ))
            .await
            .expect_or_log("Should be able to send message to client");
    }

    info!("Connecting controller client");
    scanner_state_write!(state => state.controller_conn = true);

    // Receive messages from the client and act on them
    let incoming_msgs = conn.incoming.try_for_each(|msg| {
        let tungstenite::Message::Binary(bytes) = msg else {
            return future::err(tungstenite::Error::Protocol(
                tungstenite::error::ProtocolError::ExpectedFragment(
                    tungstenite::protocol::frame::coding::Data::Binary,
                ),
            ));
        };

        let msg: ControllerToServerMsg = match bincode::deserialize(&bytes) {
            Ok(x) => x,
            Err(e) => {
                error!(
                    ?e,
                    "Unable to deserialize controller client message; disconnecting it"
                );
                return future::err(tungstenite::Error::Io(io::Error::new(
                    io::ErrorKind::InvalidData,
                    e,
                )));
            }
        };

        debug!(
            ?msg,
            "Received and deserialized message from controller client"
        );

        match msg {
            ControllerToServerMsg::DeclareClientType => {
                error!("Recieved ControllerToServerMsg::DeclareClientType after handshake");
                return future::err(tungstenite::Error::Protocol(
                    tungstenite::error::ProtocolError::ExpectedFragment(
                        tungstenite::protocol::frame::coding::Data::Binary,
                    ),
                ));
            }
            ControllerToServerMsg::EstablishConnection => {
                debug!("Controller trying to establish connection; sending AcceptConnection");
                CONTROLLER_SEND
                    .send(
                        bincode::serialize(&ServerToControllerMsg::Generic(
                            GenericServerToClientMsg::AcceptConnection,
                        ))
                        .expect_or_log("Should be able to serialize AcceptConnection"),
                    )
                    .expect_or_log("Should be able to send message down CONTROLLER_SEND");
            }
            ControllerToServerMsg::ReadyToTakePhotos {
                camera_alignment,
                pause_time_ms,
            } => {
                SEND_MESSAGE_TO_SCAN_MANAGER
                    .send(ScanManagerMsg::StartTakingPhotos {
                        camera_alignment,
                        pause_time_ms,
                    })
                    .expect_or_log(
                        "Should be able to send message down SEND_MESSAGE_TO_SCAN_MANAGER",
                    );
            }
        }

        future::ok(())
    });

    // Forward messages from CONTROLLER_SEND to the actual connection
    let recv_from_server = WatchStream::new(CONTROLLER_SEND.subscribe())
        .filter_map(|bytes| {
            future::ready(if !bytes.is_empty() {
                debug!(?bytes, "Forwarding message to controller");
                Some(Ok(tungstenite::Message::Binary(bytes)))
            } else {
                None
            })
        })
        .forward(conn.outgoing);

    SEND_MESSAGE_TO_SCAN_MANAGER
        .send(ScanManagerMsg::ControllerConnected)
        .expect_or_log("Should be able to send message down SEND_MESSAGE_TO_SCAN_MANAGER");

    pin_mut!(incoming_msgs, recv_from_server);
    future::select(incoming_msgs, recv_from_server).await;

    info!("Disconnecting controller");
    SEND_MESSAGE_TO_SCAN_MANAGER
        .send(ScanManagerMsg::ControllerDisconnected)
        .expect_or_log("Should be able to send message down SEND_MESSAGE_TO_SCAN_MANAGER");

    Ok(())
}

/// Run the server asynchronously.
#[instrument(skip_all)]
pub async fn run_server(kill_rx: oneshot::Receiver<()>) -> Result<()> {
    info!(port = env!("SCANNER_PORT"), "Initialising server");

    let tls_acceptor = match ww_shared_server_tls::make_tls_acceptor() {
        Ok(acceptor) => {
            info!("Successfully created TLS acceptor");
            Some(acceptor)
        }
        Err(error) => {
            warn!(
                ?error,
                concat!(
                    "Unable to create TLS acceptor, so using unencrypted connection.\n",
                    "If you get this warning in production, make sure to get SSL set up properly and check the README."
                )
            );
            None
        }
    };

    let listener = TcpListener::bind(concat!("0.0.0.0:", env!("SCANNER_PORT")))
        .await
        .expect_or_log("Unable to start TcpListener");

    let scan_manager_thread = thread::Builder::new()
        .name("scan-manager".to_string())
        .spawn(move || crate::scan_manager::run_scan_manager(kill_rx))
        .unwrap_or_log();

    info!("Server initialised");

    let accept_new_connections = async move {
        while let Ok((socket, addr)) = listener.accept().await {
            let tls_acceptor = tls_acceptor.clone();

            tokio::spawn(async move {
                let handle_connection_result = match tls_acceptor {
                    Some(acceptor) => {
                        let socket = acceptor
                            .accept(socket)
                            .await
                            .expect_or_log("Should be able to accept TLS connection");

                        handle_connection(ConnectionSocket::Tls(socket), addr).await
                    }

                    None => handle_connection(ConnectionSocket::Tcp(socket), addr).await,
                };

                match handle_connection_result {
                    Ok(_) => (),
                    Err(error) => error!(?error, "Error handling connection"),
                }
            });
        }
    };

    tokio::select! {
        biased;

        // If the run-effect thread is finished, then the driver has stopped for whatever reason,
        // so kill the server
        _ = async move {
            while !scan_manager_thread.is_finished() {
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        } => {
            error!("scan-manager thread has terminated prematurely. Killing server");
            return Err(Report::msg("scan-manager terminated prematurely"));
        }

        _ = accept_new_connections => {}
    }

    Ok(())
}
