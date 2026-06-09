use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use tracing::{info, warn};
use uuid::Uuid;
use wtransport::endpoint::IncomingSession;
use wtransport::endpoint::endpoint_side::Server;
use wtransport::tls::Sha256DigestFmt;
use wtransport::{Endpoint, Identity, ServerConfig};

use crate::protocol::{ClientEvent, ServerEvent, WORLD_CONFIG};
use crate::state::{SharedState, send_reliable};

const MAX_RELIABLE_EVENT_SIZE: usize = 64 * 1024;

pub async fn create_endpoint(
    port: u16,
    certificate_directory: &Path,
) -> Result<(Endpoint<Server>, String)> {
    let identity = load_or_create_identity(certificate_directory).await?;
    let certificate_hash = identity
        .certificate_chain()
        .as_slice()
        .first()
        .context("TLS identity has no certificate")?
        .hash()
        .fmt(Sha256DigestFmt::DottedHex);
    let config = ServerConfig::builder()
        .with_bind_default(port)
        .with_identity(identity)
        .keep_alive_interval(Some(Duration::from_secs(3)))
        .build();

    Ok((Endpoint::server(config)?, certificate_hash))
}

pub async fn run_server(endpoint: Endpoint<Server>, state: Arc<SharedState>) -> Result<()> {
    loop {
        let incoming_session = endpoint.accept().await;
        let state = Arc::clone(&state);

        tokio::spawn(async move {
            if let Err(error) = handle_connection(incoming_session, state).await {
                warn!(?error, "connection ended with error");
            }
        });
    }
}

async fn handle_connection(
    incoming_session: IncomingSession,
    state: Arc<SharedState>,
) -> Result<()> {
    let session_request = incoming_session.await?;
    let requested_player_number = requested_player_number(session_request.path());
    let connection = session_request.accept().await?;
    let player_id = Uuid::new_v4();
    let player_number = state.add_player(player_id, requested_player_number).await;

    if let Err(error) = send_reliable(
        &connection,
        &serde_json::to_vec(&ServerEvent::Connected {
            player_id,
            player_number,
            world: WORLD_CONFIG,
        })?,
    )
    .await
    {
        state.remove_player(player_id).await;
        return Err(error);
    }
    state.add_connection(player_id, connection.clone()).await;
    state
        .broadcast_reliable(&ServerEvent::PlayerJoined {
            player_id,
            player_number,
        })
        .await;
    info!(%player_id, player_number, remote = %connection.remote_address(), "player connected");

    let result = loop {
        tokio::select! {
            datagram = connection.receive_datagram() => {
                let datagram = match datagram {
                    Ok(datagram) => datagram,
                    Err(error) => break Err(error.into()),
                };
                match serde_json::from_slice::<ClientEvent>(&datagram) {
                    Ok(event) => state.apply_event(player_id, event).await,
                    Err(error) => warn!(%player_id, ?error, "invalid client datagram"),
                }
            }
            stream = connection.accept_uni() => {
                let mut stream = match stream {
                    Ok(stream) => stream,
                    Err(error) => break Err(error.into()),
                };
                match read_reliable_event(&mut stream).await {
                    Ok(event) => state.apply_event(player_id, event).await,
                    Err(error) => warn!(%player_id, ?error, "invalid reliable client event"),
                }
            }
            _ = connection.closed() => {
                break Ok(());
            }
        }
    };

    state.remove_player(player_id).await;
    info!(%player_id, "player disconnected");
    result
}

fn requested_player_number(path: &str) -> Option<u16> {
    path.split_once('?')?.1.split('&').find_map(|parameter| {
        let (key, value) = parameter.split_once('=')?;
        (key == "player")
            .then(|| value.parse().ok())
            .flatten()
            .filter(|number| *number > 0)
    })
}

async fn read_reliable_event(stream: &mut wtransport::RecvStream) -> Result<ClientEvent> {
    let mut payload = Vec::new();
    let mut buffer = [0_u8; 4096];

    while let Some(read) = stream.read(&mut buffer).await? {
        if payload.len() + read > MAX_RELIABLE_EVENT_SIZE {
            bail!("reliable event exceeded maximum size");
        }
        payload.extend_from_slice(&buffer[..read]);
    }

    Ok(serde_json::from_slice(&payload)?)
}

async fn load_or_create_identity(directory: &Path) -> Result<Identity> {
    let certificate_path = directory.join("cert.pem");
    let private_key_path = directory.join("key.pem");

    if certificate_path.exists() && private_key_path.exists() {
        secure_private_key(&private_key_path).await?;
        return Ok(Identity::load_pemfiles(certificate_path, private_key_path).await?);
    }

    tokio::fs::create_dir_all(directory).await?;
    let identity = Identity::self_signed(["localhost", "127.0.0.1", "::1"])?;
    identity
        .certificate_chain()
        .store_pemfile(&certificate_path)
        .await?;
    identity
        .private_key()
        .store_secret_pemfile(&private_key_path)
        .await?;
    secure_private_key(&private_key_path).await?;

    Ok(identity)
}

#[cfg(unix)]
async fn secure_private_key(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    tokio::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).await?;
    Ok(())
}

#[cfg(not(unix))]
async fn secure_private_key(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::requested_player_number;

    #[test]
    fn reads_player_number_from_session_path() {
        assert_eq!(requested_player_number("/game?player=12"), Some(12));
        assert_eq!(
            requested_player_number("/game?ignored=yes&player=128"),
            Some(128)
        );
    }

    #[test]
    fn rejects_missing_or_invalid_player_number() {
        assert_eq!(requested_player_number("/game"), None);
        assert_eq!(requested_player_number("/game?player=0"), None);
        assert_eq!(requested_player_number("/game?player=abc"), None);
        assert_eq!(requested_player_number("/game?player=999999"), None);
    }
}
