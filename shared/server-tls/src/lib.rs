//! This library just contains functions for TLS stuff that can be shared between the main server
//! and the scanner server.

#![feature(lint_reasons)]

use std::{
    fs::File,
    io::{self, BufReader},
    path::Path,
    sync::Arc,
};
use thiserror::Error;
use tokio_rustls::{
    rustls::{self, Certificate, PrivateKey},
    TlsAcceptor,
};

/// Read the file at the given path and try to read it as a list of SSL certificates.
fn load_certs(path: &Path) -> io::Result<Vec<Certificate>> {
    rustls_pemfile::certs(&mut BufReader::new(File::open(path)?))
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid certificate"))
        .map(|certs| certs.into_iter().map(Certificate).collect())
}

/// Read the file at the given path and try to read it as a list of SSL private keys.
fn load_keys(path: &Path) -> io::Result<Vec<PrivateKey>> {
    use rustls_pemfile::{read_all, Item::*};

    read_all(&mut BufReader::new(File::open(path)?))
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid keys"))
        .map(|keys| {
            keys.into_iter()
                .filter_map(|item| match item {
                    RSAKey(key) | PKCS8Key(key) | ECKey(key) => Some(PrivateKey(key)),
                    _ => None,
                })
                .collect()
        })
}

/// The error returned by [`make_tls_acceptor`].
#[derive(Debug, Error)]
#[allow(missing_docs, reason = "the #[error] attributes document the variants")]
pub enum MakeTlsAcceptorError {
    #[error("IO error: `{0:?}`")]
    Io(#[from] io::Error),

    #[error("Error from rustls: `{0:?}`")]
    Rustls(#[from] tokio_rustls::rustls::Error),

    #[error("SERVER_SSL_CERT_PATH was not defined")]
    NoCertificatePath,

    #[error("SERVER_SSL_KEY_PATH was not defined")]
    NoKeyPath,

    #[error("Zero certificates were found in the given path")]
    ZeroCertificates,

    #[error("Zero paths were found in the given path")]
    ZeroPaths,
}

/// Make a [`TlsAcceptor`] by reading the `SERVER_SSL_CERT_PATH` and `SERVER_SSL_KEY_PATH`
/// environment variables *at compile time*. Return an error if we failed to make the acceptor.
///
/// This method allows connections to be handled with a `TcpStream` or a `TlsStream` depending on
/// whether we can create a [`TlsAcceptor`]. This means that we don't need SSL when developing,
/// since the server can work unencrypted and the client can talk to localhost. But in production,
/// the client web browser normally wants an encrypted connection.
pub fn make_tls_acceptor() -> Result<TlsAcceptor, MakeTlsAcceptorError> {
    let certs = load_certs(Path::new(
        option_env!("SERVER_SSL_CERT_PATH").ok_or(MakeTlsAcceptorError::NoCertificatePath)?,
    ))?;
    let mut keys = load_keys(Path::new(
        option_env!("SERVER_SSL_KEY_PATH").ok_or(MakeTlsAcceptorError::NoKeyPath)?,
    ))?;

    if certs.is_empty() {
        return Err(MakeTlsAcceptorError::ZeroCertificates);
    }
    if keys.is_empty() {
        return Err(MakeTlsAcceptorError::ZeroPaths);
    }

    let tls_config = rustls::ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs, keys.remove(0))?;

    Ok(TlsAcceptor::from(Arc::new(tls_config)))
}
