#!/usr/bin/env bash
# Copyright 2026 Zyvor AI Labs · https://zyvor.dev
# SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

# Run the Veyron server (`veyron api-serve`: console, API) on a Mac as a launchd LaunchAgent (docs/macos-server.md).
#
#   scripts/macos-server.sh install   [--bin PATH] [--env-file FILE] [--host H] [--port N] [--tls-cert F --tls-key F] [--no-load]
#   scripts/macos-server.sh uninstall
#   scripts/macos-server.sh status
#   scripts/macos-server.sh render    [same options as install]     # print the plist, change nothing
#
# A LaunchAgent of the logged-in user, like FluxVM's and Kairon's on a Mac. Defaults: 127.0.0.1:5151, plain HTTP (Veyron.app
# accepts plain http only for localhost; give --tls-cert and --tls-key for HTTPS, and --host 0.0.0.0 to serve other machines).
#
# Secrets stay out of the plist. The agent runs `sh -c '. ENV_FILE; exec veyron api-serve ...'`, and ENV_FILE (default
# ~/.config/veyron/veyron.env) must be mode 0600 and define at least:
#
#   VEYRON_API_KEY=...                       # without it the API rejects every request
#   KUBECONFIG=/path/to/kubeconfig           # Veyron needs a Kubernetes API (with the Kairon CRDs for Kairon features)
#
# For Kairon-backed VMs on this Mac also set KAIRON_NODE_CONSOLE_TOKEN (the same value kairon-node uses) and
# VEYRON_KAIRON_NODE_URL=http://127.0.0.1:8090 (the default template looks the node up by its Kubernetes Node IP).
set -euo pipefail

LABEL="dev.zyvor.veyron"
HOME_DIR="${HOME:?HOME is not set}"
PLIST="$HOME_DIR/Library/LaunchAgents/$LABEL.plist"
LOG="$HOME_DIR/Library/Logs/veyron.log"
ENV_FILE="$HOME_DIR/.config/veyron/veyron.env"
BIN=""
HOST="127.0.0.1"
PORT="5151"
TLS_CERT=""
TLS_KEY=""
LOAD=1
REQUIRED=(VEYRON_API_KEY KUBECONFIG)

die() { echo "macos-server.sh: $*" >&2; exit 1; }

xml_escape() { sed -e 's/&/\&amp;/g' -e 's/</\&lt;/g' -e 's/>/\&gt;/g' -e 's/"/\&quot;/g'; }
# A word for sh -c, in single quotes.
shq() { printf "'%s'" "${1//\'/\'\\\'\'}"; }

usage() { sed -n '5,9p' "$0" | sed 's/^# \{0,1\}//'; }

parse_flags() {
    while [ $# -gt 0 ]; do
        case "$1" in
            --bin) BIN="${2:?--bin needs a path}"; shift 2 ;;
            --env-file) ENV_FILE="${2:?--env-file needs a path}"; shift 2 ;;
            --host) HOST="${2:?--host needs an address}"; shift 2 ;;
            --port) PORT="${2:?--port needs a number}"; shift 2 ;;
            --tls-cert) TLS_CERT="${2:?--tls-cert needs a path}"; shift 2 ;;
            --tls-key) TLS_KEY="${2:?--tls-key needs a path}"; shift 2 ;;
            --no-load) LOAD=0; shift ;;
            *) die "unknown option: $1" ;;
        esac
    done
    case "$PORT" in ''|*[!0-9]*) die "--port must be a number" ;; esac
    { [ "$PORT" -ge 1 ] && [ "$PORT" -le 65535 ]; } || die "--port must be 1 to 65535"
    case "$HOST" in *[!A-Za-z0-9.:-]*|"") die "--host must be an address or name" ;; esac
    if [ -n "$TLS_CERT" ] || [ -n "$TLS_KEY" ]; then
        { [ -f "$TLS_CERT" ] && [ -f "$TLS_KEY" ]; } || die "--tls-cert and --tls-key must both be given and exist"
    fi
}

file_mode() { stat -f '%Lp' "$1" 2>/dev/null || stat -c '%a' "$1"; }

check_env_file() {
    [ -f "$ENV_FILE" ] || die "$ENV_FILE does not exist (see the header of this script for what it must define)"
    local mode
    mode="$(file_mode "$ENV_FILE")"
    case "$mode" in
        600|400) ;;
        *) die "$ENV_FILE has mode $mode: it holds an API key, so chmod 600 it" ;;
    esac
    local k
    for k in "${REQUIRED[@]}"; do
        grep -Eq "^(export[[:space:]]+)?$k=." "$ENV_FILE" || die "$ENV_FILE does not set $k"
    done
}

resolve_bin() {
    if [ -z "$BIN" ]; then
        BIN="$(command -v veyron || true)"
    fi
    [ -n "$BIN" ] || die "veyron not found: pass --bin PATH (cargo build --release writes target/release/veyron)"
    [ -x "$BIN" ] || die "$BIN is not executable"
    case "$BIN" in /*) ;; *) BIN="$PWD/$BIN" ;; esac
}

render() {
    local script tls=""
    if [ -n "$TLS_CERT" ]; then
        tls=" --tls --tls-cert $(shq "$TLS_CERT") --tls-key $(shq "$TLS_KEY")"
    fi
    # set -a exports everything the file defines; exec keeps launchd supervising veyron itself.
    script="set -a; . $(shq "$ENV_FILE"); set +a; exec $(shq "$BIN") api-serve --host $(shq "$HOST") --port $PORT$tls"
    cat <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>$LABEL</string>
  <key>ProgramArguments</key>
  <array>
    <string>/bin/sh</string>
    <string>-c</string>
    <string>$(printf '%s' "$script" | xml_escape)</string>
  </array>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
  <key>ThrottleInterval</key><integer>30</integer>
  <key>StandardOutPath</key><string>$(printf '%s' "$LOG" | xml_escape)</string>
  <key>StandardErrorPath</key><string>$(printf '%s' "$LOG" | xml_escape)</string>
</dict>
</plist>
PLIST
}

domain() { echo "gui/$(id -u)"; }

cmd_install() {
    parse_flags "$@"
    check_env_file
    resolve_bin
    mkdir -p "$(dirname "$PLIST")" "$(dirname "$LOG")"
    render >"$PLIST"
    if command -v plutil >/dev/null 2>&1; then plutil -lint "$PLIST" >/dev/null || die "wrote an invalid plist: $PLIST"; fi
    echo "wrote $PLIST"
    [ "$LOAD" = 1 ] || return 0
    launchctl bootout "$(domain)/$LABEL" 2>/dev/null || true
    launchctl bootstrap "$(domain)" "$PLIST"
    echo "loaded $LABEL; log: $LOG"
}

cmd_uninstall() {
    launchctl bootout "$(domain)/$LABEL" 2>/dev/null || true
    if [ -f "$PLIST" ]; then rm -f "$PLIST"; echo "removed $PLIST"; else echo "not installed"; fi
}

cmd_status() {
    echo "plist:  $PLIST ($([ -f "$PLIST" ] && echo present || echo absent))"
    local out
    out="$(mktemp)"
    if launchctl print "$(domain)/$LABEL" >"$out" 2>/dev/null; then
        grep -E '^[[:space:]]*(state|pid|last exit code) =' "$out" | sed 's/^[[:space:]]*/loaded: /' || true
    else
        echo "loaded: no"
    fi
    rm -f "$out"
    echo "log:    $LOG"
}

[ "$(uname -s)" = Darwin ] || [ "${1:-}" = render ] || die "this installs a launchd agent: run it on macOS (render works anywhere)"
case "${1:-}" in
    install) shift; cmd_install "$@" ;;
    uninstall) cmd_uninstall ;;
    status) cmd_status ;;
    render) shift; parse_flags "$@"; check_env_file; resolve_bin; render ;;
    -h|--help|help|"") usage ;;
    *) die "unknown command: $1 (install, uninstall, status, render)" ;;
esac
