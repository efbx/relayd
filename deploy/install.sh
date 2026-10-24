#!/bin/sh
set -eu

repository="${RELAYD_REPO:-efbx/relayd}"
latest_release="https://github.com/$repository/releases/latest/download"
install_folder="/opt/relayd"

main() {
  mode="${1:-}"
  case "$mode" in
    server) install_server ;;
    standalone) install_desktop_app ;;
    *) fail "usage: install.sh server | standalone" ;;
  esac
}

install_server() {
  require_root
  require_command curl
  detect_this_machine
  make_scratch_folder
  download_server_release
  copy_release_into_install_folder
  sh "$install_folder/deploy/provision-tools.sh" server
  create_data_folder
  if [ "$system" = "linux" ]; then
    run_as_systemd_service
  else
    run_as_launchd_daemon
  fi
}

install_desktop_app() {
  refuse_root
  require_command curl
  detect_this_machine
  make_scratch_folder
  if [ "$system" = "linux" ]; then
    install_appimage
  else
    install_mac_app
  fi
}

download_server_release() {
  say "downloading relayd for $system-$processor"
  curl -fsSL "$latest_release/relayd-$system-$processor.tar.gz" -o "$scratch/relayd.tar.gz"
  tar -C "$scratch" -xzf "$scratch/relayd.tar.gz"
}

copy_release_into_install_folder() {
  mkdir -p "$install_folder"
  install_executable "$scratch/relayd/relayd" "$install_folder/relayd"
  rm -rf "$install_folder/relayd-ui" "$install_folder/public" "$install_folder/deploy"
  cp -R "$scratch/relayd/public" "$install_folder/public"
  cp -R "$scratch/relayd/deploy" "$install_folder/deploy"
}

create_data_folder() {
  mkdir -p "$install_folder/data"
  chmod 700 "$install_folder/data"
}

run_as_systemd_service() {
  id relayd >/dev/null 2>&1 || useradd --system --home "$install_folder" --shell /usr/sbin/nologin relayd
  chown -R relayd:relayd "$install_folder"
  cp "$install_folder/deploy/relayd.service" /etc/systemd/system/relayd.service
  systemctl daemon-reload
  systemctl enable relayd >/dev/null
  systemctl restart relayd
  say "installed; relayd runs as a systemd service"
  say "  page:   http://$(hostname -I 2>/dev/null | awk '{ print $1 }'):8080/"
  say "  logs:   journalctl -u relayd -f"
}

run_as_launchd_daemon() {
  daemon="/Library/LaunchDaemons/dev.relayd.plist"
  cp "$install_folder/deploy/relayd.plist" "$daemon"
  launchctl bootout system "$daemon" 2>/dev/null || true
  launchctl bootstrap system "$daemon"
  say "installed; relayd runs as a launchd daemon"
  say "  page:   http://localhost:8080/"
  say "  logs:   tail -f $install_folder/relayd.log"
}

install_appimage() {
  user_programs="${XDG_BIN_HOME:-$HOME/.local/bin}"
  say "downloading the relayd desktop app for $system-$processor"
  curl -fsSL "$latest_release/relayd-desktop-$system-$processor.AppImage" -o "$scratch/relayd.AppImage"
  mkdir -p "$user_programs"
  install_executable "$scratch/relayd.AppImage" "$user_programs/relayd"
  say "installed $user_programs/relayd"
}

install_mac_app() {
  applications="/Applications"
  [ -w "$applications" ] || applications="$HOME/Applications"
  say "downloading the relayd desktop app for $system-$processor"
  curl -fsSL "$latest_release/relayd-desktop-$system-$processor.dmg" -o "$scratch/relayd.dmg"
  mkdir -p "$applications" "$scratch/disk"
  hdiutil attach -nobrowse -readonly -mountpoint "$scratch/disk" "$scratch/relayd.dmg" >/dev/null
  rm -rf "$applications/RelayD.app"
  cp -R "$scratch/disk/RelayD.app" "$applications/RelayD.app"
  hdiutil detach "$scratch/disk" >/dev/null
  say "installed $applications/RelayD.app"
}

detect_this_machine() {
  case "$(uname -s)" in
    Linux) system="linux" ;;
    Darwin) system="macos" ;;
    *) fail "unsupported system: $(uname -s)" ;;
  esac
  case "$(uname -m)" in
    x86_64|amd64) processor="x86_64" ;;
    aarch64|arm64) processor="aarch64" ;;
    *) fail "unsupported processor: $(uname -m)" ;;
  esac
}

install_executable() {
  source_file="$1"
  destination="$2"
  cp "$source_file" "$destination.new"
  chmod 755 "$destination.new"
  mv -f "$destination.new" "$destination"
}

make_scratch_folder() {
  scratch="$(mktemp -d)"
  trap 'hdiutil detach "$scratch/disk" >/dev/null 2>&1 || true; rm -rf "$scratch"' EXIT
}

require_root() {
  [ "$(id -u)" -eq 0 ] || fail "server mode must run as root: curl -fsSL <url> | sudo sh -s -- server"
}

refuse_root() {
  [ "$(id -u)" -ne 0 ] || fail "standalone mode installs for the current user; run it without sudo"
}

require_command() {
  command -v "$1" >/dev/null || fail "$1 is required"
}

say() {
  printf '\033[1;33mrelayd\033[0m %s\n' "$*"
}

fail() {
  printf '\033[1;31mrelayd\033[0m %s\n' "$*" >&2
  exit 1
}

main "$@"
