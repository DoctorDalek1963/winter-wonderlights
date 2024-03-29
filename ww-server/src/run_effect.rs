//! This module provides the `run_effect` function to check the given `ClientState` and run the effect.

use super::WrappedClientState;
use lazy_static::lazy_static;
use std::time::Duration;
use tokio::sync::{broadcast, oneshot};
use tracing::{info, instrument, trace, warn};
use tracing_unwrap::ResultExt;
use ww_driver_trait::Driver;
use ww_frame::FrameType;

lazy_static! {
    /// The broadcast sender which lets you send messages to the background thread, which is
    /// running the effect itself.
    pub static ref SEND_MESSAGE_TO_RUN_EFFECT_THREAD: broadcast::Sender<ThreadMessage> = broadcast::channel(10).0;
}

/// Possible messages to send to the effect thread.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ThreadMessage {
    /// Restart the effect rendering.
    ///
    /// This could be because a new effect was selected or because the client wanted to restart the
    /// current effect.
    Restart,
}

/// Run the effect in the `state` with `tokio` and listen for messages on the
/// [`struct@SEND_MESSAGE_TO_RUN_EFFECT_THREAD`] channel. Intended to be run in a background thread.
#[instrument(skip_all)]
pub fn run_effect(client_state: WrappedClientState, kill_thread: oneshot::Receiver<()>) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap_or_log();
    let local = tokio::task::LocalSet::new();

    // Safety: This function gets run once in a background thread for the duration of the server,
    // so this call to `init()` only happens once and is thus safe.
    let mut driver = unsafe { crate::drivers::DriverWrapper::init() };

    let mut thread_message_rx = SEND_MESSAGE_TO_RUN_EFFECT_THREAD.subscribe();

    info!("Beginning tokio listen and run loop");

    /// Lock the local `client_state` for reading.
    macro_rules! read_state {
        ($name:ident => $body:expr) => {{
            let $name = client_state
                .read()
                .expect_or_log("Should be able to read from client state");
            let rv = $body;
            drop($name);
            rv
        }};
    }

    let receive_messages_and_run_effect = async move {
        loop {
            tokio::select! {
                biased;

                // First, we check if we've received a message on the channel and respond to it if so
                msg = thread_message_rx.recv() => {
                    trace!(?msg, "Received ThreadMessage");

                    match msg.expect_or_log("There should not be an error in receiving a ThreadMessage") {
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

                    if let Some(effect_name) = effect_name {
                        let mut effect = effect_name.from_file();

                        loop {
                            // This block is needed to let clippy know that we drop the state
                            // before awaiting the sleep
                            let (frame, duration) = {
                                let state = client_state
                                    .read()
                                    .expect_or_log("Should be able to read from client state");
                                let Some(config) = &state.effect_config else {
                                    drop(state);
                                    break;
                                };
                                let Some((frame, duration)) = effect.next_frame(config) else {
                                    break;
                                };
                                (frame, duration)
                            };

                            driver.display_frame(frame, read_state!(state => state.max_brightness));
                            tokio::time::sleep(duration).await;
                        }

                        driver.display_frame(FrameType::Off, read_state!(state => state.max_brightness));

                        // Pause before looping the effect
                        let pause_time_ms = read_state!(state => state.pause_time_ms);
                        tokio::time::sleep(Duration::from_millis(pause_time_ms)).await;

                        client_state.save_config();
                        info!(
                            "Looping effect {:?}",
                            read_state!(state => state.effect_name.map_or("None", |x| x.effect_name()))
                        );
                    } else {
                        driver.display_frame(FrameType::Off, read_state!(state => state.max_brightness));

                        // Don't send `FrameType::Off` constantly. `select!` takes control
                        // while we're awaiting anyway, so responding to a message will be fast
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }} => {}
            }
        }
    };

    local.block_on(&runtime, async move {
        tokio::select! {
            biased;

            // If we get told to kill this thread, then immediately return. This manual return
            // ensures that `driver` gets dropped, so that its drop impl gets correctly called
            _ = kill_thread => {
                #[allow(
                    clippy::needless_return,
                    reason = "this explicit return is clearer than an implicit one"
                )]
                return;
            }

            _ = receive_messages_and_run_effect => {}
        }
    });
}
