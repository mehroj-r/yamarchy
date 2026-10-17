mod config;
mod mpris;
mod service;
mod socket;

use anyhow::Result;
use config::Config;
use service::PlayerService;
use socket::SocketServer;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, Mutex};

#[tokio::main]
async fn main() -> Result<()> {
    println!("[yamarchy-daemon] Initializing Yandex Music daemon...");

    let config = Config::load().unwrap_or_default();
    let socket_path = config.socket_path();

    if config.auth.oauth_token.is_empty() {
        eprintln!("[yamarchy-daemon] WARNING: No OAuth token configured in ~/.config/yamarchy/config.toml");
        eprintln!("[yamarchy-daemon] Run `yamarchy auth login` to configure your token.");
    }

    let (state_tx, _) = broadcast::channel(32);
    let service = Arc::new(Mutex::new(
        PlayerService::new(config.clone(), state_tx.clone()).await?,
    ));

    // Start MPRIS2 DBus interface if enabled
    if config.daemon.enable_mpris {
        let mpris_service = Arc::clone(&service);
        tokio::spawn(async move {
            match mpris::start_mpris_server(mpris_service).await {
                Ok(_conn) => {
                    println!("[yamarchy-daemon] Registered MPRIS2 service org.mpris.MediaPlayer2.yamarchy");
                    std::future::pending::<()>().await;
                }
                Err(e) => {
                    eprintln!("[yamarchy-daemon] Failed to start MPRIS2 server: {e}");
                }
            }
        });
    }

    // Spawn ticker loop to track playback position, crossfades, and track transitions
    let ticker_service = Arc::clone(&service);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(100));
        loop {
            interval.tick().await;
            let mut svc = ticker_service.lock().await;
            svc.tick().await;
            drop(svc);
        }
    });

    println!(
        "[yamarchy-daemon] Listening on socket: {}",
        socket_path.display()
    );
    let socket_server = SocketServer::bind(&socket_path, Arc::clone(&service), state_tx)?;
    socket_server.run().await?;

    Ok(())
}
