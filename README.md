# Yamarchy 🎵

Native Yandex Music client and Omarchy Quickshell top-right panel plugin powered by a lightweight Rust daemon, MPRIS2 D-Bus integration, Unix domain socket IPC, and CLI tools.

> [!WARNING]
> **Unofficial project.** Yamarchy is not affiliated with, endorsed by, or connected to Yandex. It talks to an undocumented, reverse-engineered API that can change or break at any time. You need your own Yandex Music account (playback of full tracks requires an active Yandex Plus subscription). Use it at your own risk and in line with Yandex's terms of service.

---

## Features

- **Rust Audio Pipeline**: Low-latency playback powered by `rodio` and `symphonia` (pure-Rust decoding; output via ALSA).
- **My Vibe & Radio Streams**: Reverse-engineered rotor station streaming and feedback loop for continuous personalized playback.
- **Omarchy Status Bar Widget**: Native Quickshell popup panel on the top-right menu with live album art, track details, animated waveform, seekbar, and volume slider.
- **Full MPRIS2 Integration**: Seamless integration with system volume keys, Hyprland bindings, and media widgets via `org.mpris.MediaPlayer2.yamarchy`.
- **Fast CLI**: Instant terminal controls over Unix domain sockets with JSON support.

---

## Requirements

- Linux with ALSA (PipeWire / PulseAudio via their ALSA plugins work too)
- Rust toolchain (stable) — install via [rustup](https://rustup.rs)
- `alsa-lib` development files and `pkg-config` for building
  - Arch / Omarchy: `sudo pacman -S alsa-lib pkgconf`
  - Debian / Ubuntu: `sudo apt install libasound2-dev pkg-config`
- A D-Bus session bus (for MPRIS2)
- [Omarchy](https://omarchy.org) for the status bar plugin (the daemon and CLI work without it)

---

## Quick Start

### 1. Build & Install Binaries

```bash
cargo build --release
sudo cp target/release/yamarchy /usr/local/bin/
sudo cp target/release/yamarchy-daemon /usr/local/bin/
```

### 2. Configure Token

Yamarchy authenticates with a Yandex OAuth token. One way to get it:

1. Open <https://oauth.yandex.ru/authorize?response_type=token&client_id=23cabbbdc6cd418abb4b39c32c41195d> while signed in to Yandex and approve access.
2. You'll be redirected to a URL containing `#access_token=...`. Copy the value of `access_token` (up to the next `&`).

Then store it — the token is prompted for without echo, so it doesn't end up in your shell history:

```bash
yamarchy auth login           # paste token at the prompt
yamarchy auth status          # verify
```

The token is saved to `~/.config/yamarchy/config.toml` with `0600` permissions. Treat it like a password: anyone holding it has full access to your Yandex account's music library.

### 3. Install Omarchy Plugin

```bash
mkdir -p ~/.config/omarchy/plugins
ln -sf "$(pwd)/plugin" ~/.config/omarchy/plugins/yamarchy
omarchy restart shell
```

### 4. Run the Daemon

```bash
yamarchy daemon
```

---

## CLI Usage

```bash
# Playback Controls
yamarchy play                 # Resume playback
yamarchy pause                # Pause playback
yamarchy toggle               # Toggle play/pause
yamarchy next                 # Next track
yamarchy prev                 # Previous track

# Likes & Recommendations
yamarchy like                                            # Toggle like / add to favorites
yamarchy dislike                                         # Dislike current track and skip
yamarchy vibe user:onyourwave                            # Start My Vibe station
yamarchy vibe user:onyourwave --diversity discover       # Start My Vibe with discover diversity
yamarchy vibe-set --diversity popular --mood active      # Change active vibe configuration dynamically
yamarchy vibe activity:energy                            # Start Energy station

# Volume, Audio & Status
yamarchy volume 80            # Set volume to 80%
yamarchy crossfade on -d 4    # Enable 4s crossfade between tracks
yamarchy status               # Show playback details & progress
yamarchy status --json        # Machine-readable status
```

---

## Configuration

Settings live in `~/.config/yamarchy/config.toml`:

```toml
[auth]
oauth_token = "..."

[audio]
quality = "high"
volume = 0.8
cache_dir = "~/.cache/yamarchy"
crossfade_enabled = false
crossfade_duration = 3
crossfade_on_manual_skip = false

[daemon]
enable_mpris = true

[vibe]
station = "user:onyourwave"
diversity = "default"
mood_energy = "all"
language = "any"
```

---

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
