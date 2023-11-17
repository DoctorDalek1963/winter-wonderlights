//! This module contains ways to control the state of the server.

use futures_util::{
    pin_mut,
    stream::{SplitSink, SplitStream},
};
use std::{
    io,
    net::SocketAddr,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    net::TcpStream,
};
use tokio_rustls::server::TlsStream;
use tokio_tungstenite::{tungstenite::Message as TungsteniteMessage, WebSocketStream};
use ww_scanner_shared::CameraInfo;

/// A socket for a connection.
#[derive(Debug)]
pub enum ConnectionSocket {
    /// An unencrypted TCP stream.
    Tcp(TcpStream),

    /// An encrypted TLS stream.
    Tls(TlsStream<TcpStream>),
}

impl AsyncRead for ConnectionSocket {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Self::Tcp(tcp) => {
                pin_mut!(tcp);
                tcp.poll_read(cx, buf)
            }
            Self::Tls(tls) => {
                pin_mut!(tls);
                tls.poll_read(cx, buf)
            }
        }
    }
}

impl AsyncWrite for ConnectionSocket {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        match self.get_mut() {
            Self::Tcp(tcp) => {
                pin_mut!(tcp);
                tcp.poll_write(cx, buf)
            }
            Self::Tls(tls) => {
                pin_mut!(tls);
                tls.poll_write(cx, buf)
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        match self.get_mut() {
            Self::Tcp(tcp) => {
                pin_mut!(tcp);
                tcp.poll_flush(cx)
            }
            Self::Tls(tls) => {
                pin_mut!(tls);
                tls.poll_flush(cx)
            }
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        match self.get_mut() {
            Self::Tcp(tcp) => {
                pin_mut!(tcp);
                tcp.poll_shutdown(cx)
            }
            Self::Tls(tls) => {
                pin_mut!(tls);
                tls.poll_shutdown(cx)
            }
        }
    }
}

/// A connection to a client.
#[derive(Debug)]
pub struct Connection {
    /// The address of the client.
    pub addr: SocketAddr,

    /// The stream of messages coming from the client.
    pub incoming: SplitStream<WebSocketStream<ConnectionSocket>>,

    /// The sink of messages going to the client.
    pub outgoing: SplitSink<WebSocketStream<ConnectionSocket>, TungsteniteMessage>,
}

/// The state of the scanner as a whole - the server, camera client, and controller client.
#[derive(Debug)]
pub struct ScannerState {
    /// Is the camera connected?
    pub camera_conn: bool,

    /// Is the controller connected?
    pub controller_conn: bool,

    /// The info of the camera if it's connected.
    pub camera_info: Option<CameraInfo>,
}

impl ScannerState {
    /// Create a new instance of the scanner state with no connections.
    pub fn new() -> Self {
        Self {
            camera_conn: false,
            controller_conn: false,
            camera_info: None,
        }
    }

    /// Disconnect the camera client.
    pub fn disconnect_camera(&mut self) {
        self.camera_conn = false;
        self.camera_info = None;
    }

    /// Disconnect the controller client.
    pub fn disconnect_controller(&mut self) {
        self.controller_conn = false;
    }
}
