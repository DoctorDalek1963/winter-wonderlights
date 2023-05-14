//! This binary crate just runs the server for Winter WonderLights, currently just to test
//! features.

mod drivers;

use color_eyre::Result;
use std::{
    ops::Deref,
    sync::{Arc, RwLock},
    thread,
    time::Duration,
};
use tiny_http::{Header, Request, Response};
use tracing::{debug, error, info, instrument, trace, warn};
use tracing_subscriber::{filter::LevelFilter, fmt::Layer, prelude::*, EnvFilter};
use tracing_unwrap::ResultExt;
use ww_effects::{traits::get_config_filename, EffectConfigNameList};
use ww_shared_msgs::{ClientState, ClientToServerMsg, ServerToClientMsg};

/// The `.expect()` error message for serializing a [`ServerToClientMsg`].
const EXPECT_SERIALIZE_MSG: &str = "Serializing a ServerToClientMsg should never fail";

/// A simple wrapper struct to hold the client state.
#[derive(Clone, Debug)]
struct WrappedClientState(Arc<RwLock<ClientState>>);

impl WrappedClientState {
    /// Save the config of the client state.
    fn save_config(&self) {
        debug!("Saving config to file");

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

/// Create a header that will allow the client to function properly without CORS getting in the way.
fn no_cors_header() -> Header {
    Header {
        field: "Access-Control-Allow-Origin"
            .parse()
            .expect("This &'static str should parse just fine"),
        value: "*"
            .parse()
            .expect("This &'static str should parse just fine"),
    }
}

#[instrument(skip_all, fields(addr = ?req.remote_addr()))]
fn handle_request(mut req: Request, client_state: &WrappedClientState) -> Result<()> {
    debug!("Received a new request");

    trace!(?req);

    let mut body = String::new();
    req.as_reader().read_to_string(&mut body)?;
    let msg: ClientToServerMsg = ron::from_str(&body)?;

    trace!(?msg);

    let mut client = client_state
        .write()
        .expect_or_log("Should be able to write to client state");

    macro_rules! respond_update_client_state {
        () => {
            req.respond(
                Response::from_string(
                    ron::to_string(&ServerToClientMsg::UpdateClientState(client.clone()))
                        .expect(EXPECT_SERIALIZE_MSG),
                )
                .with_header(no_cors_header()),
            )?
        };
    }

    match msg {
        ClientToServerMsg::RequestUpdate => {
            info!("Client requesting update");

            respond_update_client_state!();
        }
        ClientToServerMsg::UpdateConfig(new_config) => {
            info!("Client requesting config change");
            client.effect_config = Some(new_config);
            info!(?client);
            respond_update_client_state!();
        }
        ClientToServerMsg::ChangeEffect(_effect) => {
            todo!("Implement some sort of effect manager")
        }
    };

    Ok(())
}

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
        .expect("Setting the global default for tracing should be okay");
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

    let client_state = WrappedClientState(Arc::new(RwLock::new(ClientState {
        effect_config: Some(EffectConfigNameList::MovingPlaneConfig.config_from_file()),
    })));

    // Save the effect config every 10 seconds
    thread::spawn({
        let state = client_state.clone();
        move || loop {
            state.save_config();
            thread::sleep(Duration::from_secs(10));
        }
    });

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
