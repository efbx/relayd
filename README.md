# RelayD

[![CI](https://github.com/efbx/relayd/actions/workflows/ci.yml/badge.svg)](https://github.com/efbx/relayd/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A reverse proxy with a built-in admin page, published through a Cloudflare tunnel.

## Features

- **Path routing**: each path prefix forwards to its own upstream.
- **Headers**: add or strip headers per target.
- **Live monitor**: every request appears in the admin page as it happens, with the time it was sent.
- **Stored contents**: the newest requests keep their headers and bodies; a click on a row shows them.
- **Tunnel**: a quick `trycloudflare.com` address or a custom domain.
- **Per-target VPN**: each target can send its traffic through its own WireGuard tunnel.
- **Two modes**: a server with a web admin page, or a desktop app.

## Installation

| Mode | Runs on | Command |
|---|---|---|
| Server | Linux with systemd, or macOS | `curl -fsSL https://github.com/efbx/relayd/releases/latest/download/install.sh \| sudo sh -s -- server` |
| Proxmox | the Proxmox host, as root | `curl -fsSL https://github.com/efbx/relayd/releases/latest/download/proxmox.sh \| sh` |
| Desktop app | macOS or Linux, without `sudo` | `curl -fsSL https://github.com/efbx/relayd/releases/latest/download/install.sh \| sh -s -- standalone` |

All three support x86_64 and arm64.

### Server

The script installs relayd to `/opt/relayd`, installs `cloudflared` and `wireproxy` to `/usr/local/bin`, and starts relayd
as a systemd service on Linux or a launchd daemon on macOS.

| | |
|---|---|
| Admin page | `http://<host>:8080/` |
| Logs | `journalctl -u relayd -f` on Linux, `/opt/relayd/relayd.log` on macOS |
| Update | run the install command again |

### Proxmox

The script creates a container, installs relayd in it as a server, and prints the admin page address.

| Container | |
|---|---|
| System | Debian 13, unprivileged |
| Features | nesting, the Proxmox default for unprivileged containers |
| CPU | 1 core |
| Memory | 256 MB |
| Disk | 1 GB |
| Network | DHCP, starts on boot |

| Variable | Default |
|---|---|
| `RELAYD_CTID` | the next free ID |
| `RELAYD_STORAGE` | the first active storage for container disks |
| `RELAYD_BRIDGE` | `vmbr0` |
| `RELAYD_TEMPLATE_STORAGE` | `local` |

To update, run in the host shell with the container ID:

```sh
pct exec <id> -- sh -c 'curl -fsSL https://github.com/efbx/relayd/releases/latest/download/install.sh | sh -s -- server'
```

### Desktop app

The script installs `RelayD.app` to `/Applications` on macOS, or the AppImage to `~/.local/bin/relayd` on Linux. The app
includes `cloudflared` and `wireproxy`. The same files are on the
[latest release](https://github.com/efbx/relayd/releases/latest) page. To update, run the install command
again.

## Configuration

Settings are edited on the admin page and stored in `relayd.ron` in the data folder. Tunnel changes apply on restart.

The monitor keeps the last 10,000 requests. "Stored contents" sets how many of the newest ones also keep their headers and
bodies, 10 by default and 0 for off, with at most 1 MiB per body; the rest of a longer body streams through unrecorded.
Credentials are stored redacted: the `authorization`, `proxy-authorization`, `x-api-key`, `cookie` and `set-cookie`
headers, and every header a `set` rule injects.

| Mode | Data folder |
|---|---|
| Server | `/opt/relayd/data`, or `data/` in the working directory when run by hand |
| Desktop, macOS | `~/Library/Application Support/RelayD` |
| Desktop, Linux | `~/.local/share/RelayD` |

The server listens on the address from the `IP` and `PORT` environment variables, which the installed service sets to
`0.0.0.0` and `8080`.

### Tunnel

| Mode | Behavior |
|---|---|
| Quick, the default | A free random `trycloudflare.com` address that changes on every restart. |
| Own domain | A Cloudflare tunnel on a custom domain. Create the tunnel in the Cloudflare dashboard under Zero Trust, Networks, Tunnels, point a public hostname at the address shown on the Settings page, and paste the tunnel token there. |
| Off | No tunnel. The proxy is reachable on the local network only. |

The active tunnel address is shown in the admin page header and printed in the log.

## Uninstall

| Mode | Command |
|---|---|
| Server, Linux | `sudo systemctl disable --now relayd && sudo rm -rf /opt/relayd /etc/systemd/system/relayd.service && sudo userdel relayd` |
| Server, macOS | `sudo launchctl bootout system /Library/LaunchDaemons/dev.relayd.plist && sudo rm -rf /opt/relayd /Library/LaunchDaemons/dev.relayd.plist` |
| Proxmox | `pct stop <id> && pct destroy <id>` |
| Desktop app | delete `RelayD.app` or `~/.local/bin/relayd`, and the data folder |

The server install leaves `cloudflared` and `wireproxy` in `/usr/local/bin`; delete them too if nothing else uses them.

## Building from source

### Requirements

```sh
rustup target add wasm32-unknown-unknown
cargo install dioxus-cli --version 0.7.10 --locked
```

A Linux desktop build also needs the system webview libraries. On Debian and Ubuntu:

```sh
sudo apt-get install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev \
  libayatana-appindicator3-dev librsvg2-dev lld
```

### Server

```sh
dx bundle --package relayd-app --platform web --fullstack --release
cd target/dx/relayd/release/web
./server
```

The output contains the `server` binary and the `public/` folder it serves. Tunnels and VPNs need `cloudflared` and
`wireproxy` on `PATH`, which `sudo sh deploy/provision-tools.sh server` installs. Without them relayd starts with both
features unavailable.

### Desktop app

```sh
sh deploy/provision-tools.sh standalone
dx bundle --package relayd-app --desktop --release --package-types macos --package-types dmg
```

`provision-tools.sh standalone` downloads `cloudflared` and `wireproxy` for the build machine, and the bundle includes
them. On Linux, use `--package-types appimage`.

### Tests

```sh
cargo test --workspace --features relayd-app/server
cargo test --workspace --features relayd-app/standalone
```

## License

[MIT](LICENSE)
