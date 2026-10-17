# Contributing

Thanks for your interest! Bug reports, ideas and pull requests are welcome.

## Getting Started

You need the stable Rust toolchain ([rustup](https://rustup.rs)), the ALSA development files and `pkg-config`
(`sudo pacman -S alsa-lib pkgconf` on Arch / Omarchy, `sudo apt install libasound2-dev pkg-config` on Debian / Ubuntu),
and a Yandex Music account for anything that talks to the API.

```bash
cargo build --workspace
cargo run --bin yamarchy -- auth login   # paste your OAuth token
cargo run --bin yamarchy-daemon          # in one terminal
cargo run --bin yamarchy -- status       # in another
```

To work on the panel, link `plugin/` into Omarchy and restart the shell, as in the README's Quick Start.

## Before Opening a Pull Request

CI runs these on every pull request:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

- Keep changes focused; one topic per pull request.
- Add or update tests in the `#[cfg(test)]` module next to the code when behavior changes. Tests must not need a
  Yandex account or network access.
- New API calls go through `yamarchy-core` (`models.rs` and `client.rs`). [`AGENTS.md`](AGENTS.md) documents the
  reverse-engineered API and the IPC protocol; update it along with them.
- New `config.toml` settings and CLI commands go in the README.
- Never commit OAuth tokens or your `config.toml`.
- Commit messages use a short imperative subject (`Add CI workflow and systemd user unit`).
- Pull requests target `main` and are squash-merged. Releases are cut by pushing a `vX.Y.Z` tag that matches the
  version in `Cargo.toml` and `plugin/manifest.json`; the Release workflow builds and publishes the binaries.

## Questions and Security

Ask questions in [Discussions](https://github.com/mehroj-r/yamarchy/discussions). Report vulnerabilities
privately as described in [`SECURITY.md`](SECURITY.md).

By participating you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md).
