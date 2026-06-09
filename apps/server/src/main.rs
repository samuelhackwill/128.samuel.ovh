mod network;
mod protocol;
mod state;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tokio::time::MissedTickBehavior;
use tracing::info;
use tracing_subscriber::EnvFilter;

use crate::network::{create_endpoint, run_server};
use crate::state::SharedState;

const TICK_RATE: u64 = 60;
const SNAPSHOT_RATE: u64 = 30;

#[tokio::main]
async fn main() -> Result<()> {
    init_logging();

    let port = std::env::var("PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(4433);
    let certificate_directory = std::env::var("CERT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("certs"));

    let (endpoint, certificate_hash) = create_endpoint(port, &certificate_directory).await?;
    let state = Arc::new(SharedState::default());

    info!(port, "128 WebTransport server listening");
    info!("Set VITE_WEBTRANSPORT_CERT_HASH={certificate_hash} when running the web client");

    tokio::spawn(run_simulation(Arc::clone(&state)));
    run_server(endpoint, state).await
}

async fn run_simulation(state: Arc<SharedState>) {
    let mut interval = tokio::time::interval(Duration::from_secs_f64(1.0 / TICK_RATE as f64));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        interval.tick().await;
        let tick = state.advance_tick().await;

        if tick % (TICK_RATE / SNAPSHOT_RATE) == 0 {
            state.broadcast_snapshot().await;
        }
    }
}

fn init_logging() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("server_128=info")),
        )
        .init();
}
