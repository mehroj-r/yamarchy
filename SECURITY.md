# Security Policy

## Supported Versions

Only the latest release and the `main` branch receive security fixes.

## Reporting a Vulnerability

Please **do not** open a public issue for security problems.

Report privately through GitHub: **Security → Report a vulnerability**
(<https://github.com/mehroj-r/yamarchy/security/advisories/new>).

Include what you found, how to reproduce it, and the impact you expect. You should get a reply within a few days;
once a fix is released, the advisory is published with credit unless you prefer otherwise.

Particularly relevant areas: the Yandex OAuth token (read without echo by `yamarchy auth login`, stored in
`~/.config/yamarchy/config.toml` with `0600` permissions, and sent to the Yandex API), the daemon's IPC socket
(`$XDG_RUNTIME_DIR/yamarchy.sock`, or `/tmp/yamarchy.sock` without a runtime directory), which takes playback, like
and station commands from any process that can connect to it, the MPRIS2 service on the session bus, and the signed
stream URLs the daemon builds and fetches from hosts named in API responses.
