mod client;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use client::{default_socket_path, send_command, CliCommand};
use std::path::PathBuf;
use yamarchy_core::client::YandexMusicClient;

fn parse_bool_or_toggle(s: &str) -> Result<bool, String> {
    match s.to_lowercase().as_str() {
        "on" | "true" | "1" | "yes" | "enable" | "enabled" => Ok(true),
        "off" | "false" | "0" | "no" | "disable" | "disabled" => Ok(false),
        other => Err(format!(
            "Invalid boolean/toggle value '{other}'. Use on/off, true/false, or enable/disable."
        )),
    }
}

#[derive(Parser)]
#[command(name = "yamarchy", author, version, about = "Yandex Music CLI and daemon controller", long_about = None)]
struct Cli {
    /// Socket path override
    #[arg(short, long, global = true)]
    socket: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the background playback daemon
    Daemon,
    /// Get current playback status
    Status {
        /// Output raw JSON status
        #[arg(long)]
        json: bool,
    },
    /// Resume playback
    Play,
    /// Pause playback
    Pause,
    /// Toggle play/pause
    Toggle,
    /// Skip to next track
    Next,
    /// Return to previous track
    Prev,
    /// Toggle like status for current track
    Like,
    /// Dislike current track and skip
    Dislike,
    /// Set volume (0-100)
    Volume {
        #[arg(value_parser = clap::value_parser!(u32))]
        level: u32,
    },
    /// Configure crossfading between tracks
    Crossfade {
        /// Enable or disable crossfade (on/off, true/false, enable/disable)
        #[arg(value_parser = parse_bool_or_toggle)]
        state: Option<bool>,
        /// Crossfade duration in seconds (1-5)
        #[arg(short, long, value_parser = clap::value_parser!(u32))]
        duration: Option<u32>,
        /// Enable or disable crossfade on manual skip
        #[arg(long, value_parser = parse_bool_or_toggle)]
        on_skip: Option<bool>,
    },
    /// General audio and playback settings
    Settings {
        /// Toggle crossfading (on/off)
        #[arg(long, value_parser = parse_bool_or_toggle)]
        crossfade: Option<bool>,
        /// Crossfade duration in seconds (1-5)
        #[arg(long, value_parser = clap::value_parser!(u32))]
        duration: Option<u32>,
        /// Crossfade on manual skip (on/off)
        #[arg(long, value_parser = parse_bool_or_toggle)]
        crossfade_on_skip: Option<bool>,
    },
    /// Start playing a station / My Vibe
    Vibe {
        /// Station identifier (default: user:onyourwave)
        station: Option<String>,
        /// Diversity: default | discover | popular
        #[arg(short, long)]
        diversity: Option<String>,
        /// Mood / Energy: all | fun | active | calm | sad
        #[arg(short, long)]
        mood: Option<String>,
        /// Language: any | not-russian | russian | without-words
        #[arg(short, long)]
        language: Option<String>,
    },
    /// Configure vibe parameters (diversity, mood, language)
    VibeSet {
        /// Diversity: default | discover | popular
        #[arg(short, long)]
        diversity: Option<String>,
        /// Mood / Energy: all | fun | active | calm | sad
        #[arg(short, long)]
        mood: Option<String>,
        /// Language: any | not-russian | russian | without-words
        #[arg(short, long)]
        language: Option<String>,
    },
    /// Manage authentication and tokens
    Auth {
        #[command(subcommand)]
        sub: AuthCommands,
    },
}

#[derive(Subcommand)]
enum AuthCommands {
    /// Store OAuth token (prompted for, or read from stdin when piped)
    Login,
    /// Check current account status
    Status,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let socket_path = cli.socket.unwrap_or_else(default_socket_path);

    match cli.command {
        Commands::Daemon => {
            println!("Starting yamarchy-daemon...");
            let exe_dir = std::env::current_exe()?
                .parent()
                .context("No parent directory for binary")?
                .to_path_buf();
            let daemon_binary = exe_dir.join("yamarchy-daemon");
            let mut child = std::process::Command::new(if daemon_binary.exists() {
                daemon_binary
            } else {
                PathBuf::from("yamarchy-daemon")
            })
            .spawn()
            .context("Failed to spawn yamarchy-daemon process")?;
            child.wait()?;
        }
        Commands::Status { json } => {
            let resp = send_command(&socket_path, CliCommand::GetStatus).await?;
            if let Some(state) = resp.state {
                if json {
                    println!("{}", serde_json::to_string_pretty(&state)?);
                } else {
                    println!("Status:   {:?}", state.status);
                    if let Some(track) = state.current_track {
                        let liked_str = if track.liked { " [Liked ❤️]" } else { "" };
                        println!("Track:    {} - {}{}", track.artist, track.title, liked_str);
                        println!("Album:    {}", track.album);
                        let pos_sec = state.position_ms / 1000;
                        let dur_sec = state.duration_ms / 1000;
                        println!(
                            "Progress: {:02}:{:02} / {:02}:{:02} ({:.1}%)",
                            pos_sec / 60,
                            pos_sec % 60,
                            dur_sec / 60,
                            dur_sec % 60,
                            if state.duration_ms > 0 {
                                (state.position_ms as f64 / state.duration_ms as f64) * 100.0
                            } else {
                                0.0
                            }
                        );
                    } else {
                        println!("Track:    No track currently loaded");
                    }
                    println!("Volume:   {:.0}%", state.volume * 100.0);
                    if let Some(st) = state.station {
                        println!("Station:  {}", st);
                    }
                    println!(
                        "Vibe:     Diversity: {}, Mood: {}, Language: {}",
                        state.vibe_settings.diversity,
                        state.vibe_settings.mood_energy,
                        state.vibe_settings.language
                    );
                    let xf_str = if state.audio_settings.crossfade_enabled {
                        format!(
                            "Enabled ({}s, on-skip: {})",
                            state.audio_settings.crossfade_duration,
                            if state.audio_settings.crossfade_on_manual_skip {
                                "on"
                            } else {
                                "off"
                            }
                        )
                    } else {
                        "Disabled".to_string()
                    };
                    println!("Audio:    Crossfade: {}", xf_str);
                }
            } else {
                println!("No state returned from daemon.");
            }
        }
        Commands::Play => {
            let _ = send_command(&socket_path, CliCommand::Play).await?;
            println!("Playback resumed");
        }
        Commands::Pause => {
            let _ = send_command(&socket_path, CliCommand::Pause).await?;
            println!("Playback paused");
        }
        Commands::Toggle => {
            let resp = send_command(&socket_path, CliCommand::Toggle).await?;
            if let Some(state) = resp.state {
                println!("Playback state: {:?}", state.status);
            }
        }
        Commands::Next => {
            let resp = send_command(&socket_path, CliCommand::Next).await?;
            if let Some(state) = resp.state {
                if let Some(track) = state.current_track {
                    println!("Now playing: {} - {}", track.artist, track.title);
                }
            }
        }
        Commands::Prev => {
            let resp = send_command(&socket_path, CliCommand::Prev).await?;
            if let Some(state) = resp.state {
                if let Some(track) = state.current_track {
                    println!("Now playing: {} - {}", track.artist, track.title);
                }
            }
        }
        Commands::Like => {
            let resp = send_command(&socket_path, CliCommand::ToggleLike).await?;
            if let Some(state) = resp.state {
                println!(
                    "Track like status: {}",
                    if state.liked {
                        "Liked ❤️"
                    } else {
                        "Unliked"
                    }
                );
            }
        }
        Commands::Dislike => {
            let _ = send_command(&socket_path, CliCommand::Dislike).await?;
            println!("Track disliked and skipped");
        }
        Commands::Crossfade {
            state,
            duration,
            on_skip,
        } => {
            let resp = send_command(
                &socket_path,
                CliCommand::SetAudioSettings {
                    crossfade_enabled: state,
                    crossfade_duration: duration,
                    crossfade_on_manual_skip: on_skip,
                },
            )
            .await?;
            if let Some(st) = resp.state {
                let xf = st.audio_settings;
                let status_str = if xf.crossfade_enabled {
                    format!(
                        "Enabled (duration: {}s, on-skip: {})",
                        xf.crossfade_duration,
                        if xf.crossfade_on_manual_skip {
                            "on"
                        } else {
                            "off"
                        }
                    )
                } else {
                    "Disabled".to_string()
                };
                println!("Crossfade settings updated: {}", status_str);
            }
        }
        Commands::Settings {
            crossfade,
            duration,
            crossfade_on_skip,
        } => {
            let resp = send_command(
                &socket_path,
                CliCommand::SetAudioSettings {
                    crossfade_enabled: crossfade,
                    crossfade_duration: duration,
                    crossfade_on_manual_skip: crossfade_on_skip,
                },
            )
            .await?;
            if let Some(st) = resp.state {
                let xf = st.audio_settings;
                println!(
                    "Audio settings: crossfade={}, duration={}s, on-skip={}",
                    if xf.crossfade_enabled { "on" } else { "off" },
                    xf.crossfade_duration,
                    if xf.crossfade_on_manual_skip {
                        "on"
                    } else {
                        "off"
                    }
                );
            }
        }
        Commands::Volume { level } => {
            let vol = (level.min(100) as f32) / 100.0;
            let _ = send_command(&socket_path, CliCommand::SetVolume { volume: vol }).await?;
            println!("Volume set to {level}%");
        }
        Commands::Vibe {
            station,
            diversity,
            mood,
            language,
        } => {
            let resp = send_command(
                &socket_path,
                CliCommand::PlayVibe {
                    station,
                    diversity,
                    mood_energy: mood,
                    language,
                },
            )
            .await?;
            if let Some(state) = resp.state {
                if let Some(track) = state.current_track {
                    println!(
                        "Started station. Now playing: {} - {}",
                        track.artist, track.title
                    );
                }
            }
        }
        Commands::VibeSet {
            diversity,
            mood,
            language,
        } => {
            let resp = send_command(
                &socket_path,
                CliCommand::SetVibeSettings {
                    diversity,
                    mood_energy: mood,
                    language,
                },
            )
            .await?;
            if let Some(state) = resp.state {
                println!(
                    "Vibe settings updated: diversity={}, mood={}, language={}",
                    state.vibe_settings.diversity,
                    state.vibe_settings.mood_energy,
                    state.vibe_settings.language
                );
                if let Some(track) = state.current_track {
                    println!("Now playing: {} - {}", track.artist, track.title);
                }
            }
        }
        Commands::Auth { sub } => match sub {
            AuthCommands::Login => {
                let token = read_token()?;
                println!("Validating token with Yandex Music API...");
                let acc = YandexMusicClient::new(token.clone())
                    .get_account_status()
                    .await
                    .context("Authentication failed")?;
                let name = acc
                    .account
                    .as_ref()
                    .and_then(|a| a.display_name.clone().or(a.login.clone()))
                    .unwrap_or_else(|| "User".to_string());
                println!("Authenticated successfully as: {name}");

                let config_file = config_path()?;
                let mut config = load_config_table(&config_file)?;
                let auth = config
                    .entry("auth")
                    .or_insert_with(|| toml::Value::Table(toml::Table::new()))
                    .as_table_mut()
                    .context("[auth] in config is not a table")?;
                auth.insert("oauth_token".into(), toml::Value::String(token));
                save_config_table(&config_file, &config)?;
                println!("Saved configuration to {}", config_file.display());
            }
            AuthCommands::Status => {
                let config_file = config_path()?;
                let config = load_config_table(&config_file)?;
                let token = config
                    .get("auth")
                    .and_then(|a| a.get("oauth_token"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("");
                if token.is_empty() {
                    anyhow::bail!("Not logged in. Run `yamarchy auth login`.");
                }

                let acc = YandexMusicClient::new(token)
                    .get_account_status()
                    .await
                    .context("Token verification failed")?;
                let name = acc
                    .account
                    .as_ref()
                    .and_then(|a| a.display_name.clone().or(a.login.clone()))
                    .unwrap_or_else(|| "User".to_string());
                let uid = acc.account.as_ref().and_then(|a| a.uid).unwrap_or_default();
                println!("Authenticated as: {name} (UID: {uid})");
            }
        },
    }

    Ok(())
}

fn config_path() -> Result<PathBuf> {
    Ok(dirs::config_dir()
        .context("Could not determine config directory")?
        .join("yamarchy")
        .join("config.toml"))
}

/// Loads the config as a raw TOML table so that editing one key keeps every
/// other setting (vibe, crossfade, ...) the daemon has written.
fn load_config_table(path: &std::path::Path) -> Result<toml::Table> {
    if !path.exists() {
        return Ok(toml::Table::new());
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config at {}", path.display()))?;
    content
        .parse()
        .with_context(|| format!("Failed to parse config at {}", path.display()))
}

/// Saves the config readable only by the owner (0600), since it holds the token.
fn save_config_table(path: &std::path::Path, config: &toml::Table) -> Result<()> {
    use std::io::Write;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
        .with_context(|| format!("Failed to write config to {}", path.display()))?;
    file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    file.write_all(toml::to_string_pretty(config)?.as_bytes())?;
    Ok(())
}

/// Reads the OAuth token without echoing it, or from stdin when piped, so it
/// never ends up in shell history or the process list.
fn read_token() -> Result<String> {
    use std::io::IsTerminal;

    let token = if std::io::stdin().is_terminal() {
        rpassword::prompt_password("Yandex Music OAuth token: ")?
    } else {
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        line
    };
    let token = token.trim().to_string();
    if token.is_empty() {
        anyhow::bail!("No token provided");
    }
    Ok(token)
}
