# AGENTS.md — Yamarchy Development & Reverse-Engineering Guide

This document defines architecture patterns, reverse-engineering methodology, API models, and workflows for agents and developers working on `yamarchy`.

---

## 1. Project Overview

`yamarchy` is an Omarchy desktop integration and native client for Yandex Music consisting of:

- **`yamarchy-core`**: Reverse-engineered Yandex Music Web & Mobile API client (auth, track metadata, XML download resolver, rotor stream queues, feedback).
- **`yamarchy-player`**: Pure Rust audio engine using `rodio` + `symphonia` with gapless buffering.
- **`yamarchy-daemon`**: Background daemon providing Unix domain socket IPC (`$XDG_RUNTIME_DIR/yamarchy.sock`), MPRIS2 D-Bus service, queue scheduler, and state synchronization.
- **`yamarchy-cli`**: Terminal control utility (`yamarchy play`, `pause`, `next`, `prev`, `like`, `vibe`, `status`, `auth login`).
- **`plugin/`**: Quickshell QML panel plugin living on the top-right Omarchy status bar.

---

## 2. Reverse-Engineered Yandex Music API Reference

All API calls target base URL: `https://api.music.yandex.net`

### 2.1 Headers & Authentication

- Header: `Authorization: OAuth <OAUTH_TOKEN>`
- User-Agent: `YandexMusicAndroid/24022411 (Android 14; Pixel 8 Pro)`

### 2.2 Account & Status

- `GET /account/status`
  - Returns current user account `uid`, `login`, `displayName`, and active subscription status.

### 2.3 Audio Stream URL Generation (MD5 Sign Scheme)

1. `GET /tracks/{track_id}/download-info`
   - Returns list of codecs and bitrates with `downloadInfoUrl`.
2. `GET {downloadInfoUrl}`
   - Returns XML containing `<host>`, `<path>`, `<ts>`, `<s>`.
3. Compute MD5 checksum:
   - Secret salt: `X-Retpath-Y`
   - `sign = md5(salt + path.substr(1) + s)`
4. Direct Stream URL:
   - `https://{host}/get-mp3/{sign}/{ts}{path}`

### 2.4 My Vibe / Rotor Radio Stations

- `GET /rotor/station/{station_id}/tracks?diversity={diversity}&moodEnergy={mood_energy}&language={language}&include-tracks-in-storage=true`
  - Returns batch sequence containing track list, `batchId`, and station metadata.
  - Standard station IDs: `user:onyourwave` (My Vibe), `activity:energy`, `mood:calm`, `activity:work`.
  - Vibe configurations:
    - Diversity: `default` (Favorite / Моё), `discover` (Discover / Незнакомое), `popular` (Popular / Популярное)
    - Mood / Energy: `all`, `active`, `fun`, `calm`, `sad`
    - Language: `any`, `not-russian` (Foreign), `russian`, `without-words` (Instrumental)
- `POST /rotor/station/{station_id}/feedback`
  - Feedback events: `radioStarted`, `trackStarted`, `trackFinished`, `skip`, `like`, `dislike`.
  - Body schema:

    ```json
    {
      "type": "trackStarted",
      "timestamp": "1700000000.000Z",
      "trackId": "12345678",
      "totalPlayedSeconds": 15.2,
      "batchId": "..."
    }
    ```

### 2.5 Likes & User Library

- `POST /users/{uid}/likes/tracks/add-multiple` (form parameter `track-ids={id}`)
- `POST /users/{uid}/likes/tracks/remove` (form parameter `track-ids={id}`)
- `GET /users/{uid}/likes/tracks`

---

## 3. IPC & Quickshell Integration

### 3.1 Unix Domain Socket Protocol

- Socket path: `$XDG_RUNTIME_DIR/yamarchy.sock` (fallback: `/tmp/yamarchy.sock`).
- Messages are newline-terminated JSON objects:
  - Command: `{"command": "play"}`, `{"command": "set_volume", "volume": 0.8}`, `{"command": "play_vibe", "station": "user:onyourwave"}`.
  - Response / Broadcast:

    ```json
    {
      "status": "ok",
      "state": {
        "status": "playing",
        "current_track": {
          "id": "...",
          "title": "...",
          "artist": "...",
          "album": "...",
          "duration_ms": 210000,
          "cover_url": "...",
          "liked": true
        },
        "position_ms": 45000,
        "duration_ms": 210000,
        "volume": 0.8,
        "station": "user:onyourwave",
        "queue": [],
        "liked": true
      }
    }
    ```

### 3.2 Linking Plugin to Omarchy Shell

```bash
ln -sf "$(pwd)/plugin" ~/.config/omarchy/plugins/yamarchy
# Rescan / reload shell plugins:
omarchy restart shell
```

---

## 4. Build, Verification & Testing Commands

```bash
# Workspace compilation check
cargo check --workspace

# Release build
cargo build --release --workspace

# Run daemon
cargo run --bin yamarchy-daemon

# Run CLI commands
cargo run --bin yamarchy -- vibe user:onyourwave --diversity discover --mood active
cargo run --bin yamarchy -- vibe-set --diversity popular --mood fun --language russian
cargo run --bin yamarchy -- status
cargo run --bin yamarchy -- toggle
```

---

## 5. Coding & Safety Rules

- Never commit user OAuth tokens to git.
- Keep audio stream fetching non-blocking on tokio worker threads.
- Audio engine (`rodio::Sink`) must be managed safely without blocking async event loops.
- All new network requests should be mapped in `yamarchy-core/src/models.rs` and `yamarchy-core/src/client.rs`.
