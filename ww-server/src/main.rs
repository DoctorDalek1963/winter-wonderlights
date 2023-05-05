//! This binary crate just runs the server for Winter WonderLights, currently just to test
//! features.

mod drivers;

//use self::drivers::DebugDriver;
//use ww_effects::EffectList;
use color_eyre::Result;
use tiny_http::Request;
use tracing::{debug, info, instrument, warn};
use tracing_subscriber::{filter::LevelFilter, fmt::Layer, prelude::*, EnvFilter};
use tracing_unwrap::ResultExt;

#[instrument(skip_all, fields(addr = ?req.remote_addr()))]
async fn handle_request(mut req: Request) -> Result<()> {
    info!("Received a new request");

    debug!(?req);

    todo!("Implement this")
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

#[tokio::main]
#[instrument]
async fn main() {
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

    info!("Server initialised");

    for req in server.incoming_requests() {
        tokio::spawn(handle_request(req));
    }

    info!("Server socket has shut down. Terminating server");

    //let mut driver = DebugDriver { lights_num: 500 };
    //EffectList::DebugBinaryIndex.create_run_method()(&mut driver).await;
}
