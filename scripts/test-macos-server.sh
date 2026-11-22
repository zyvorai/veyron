#!/usr/bin/env bash
# Copyright 2026 Zyvor AI Labs · https://zyvor.dev
# SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

# Offline test of scripts/macos-server.sh: renders the LaunchAgent for a throwaway HOME and checks it. Loads nothing into
# launchd and needs no Kubernetes, so it runs on any host with bash (plutil is used when present).
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCRIPT="$HERE/macos-server.sh"
T="$(mktemp -d)"
trap 'rm -rf "$T"' EXIT
export HOME="$T/home with space"
mkdir -p "$HOME/.config/veyron"
FAILS=0
ok() { echo "  ok    $1"; }
bad() { echo "  FAIL  $1"; FAILS=$((FAILS + 1)); }
expect() { if eval "$2"; then ok "$1"; else bad "$1"; fi; }
# refuses NAME MESSAGE CMD...: the command fails and says MESSAGE (output captured first: grep -q would SIGPIPE it).
refuses() {
    local name="$1" msg="$2" out rc
    shift 2
    out="$("$@" 2>&1)"; rc=$?
    if [ "$rc" -ne 0 ] && [[ "$out" == *"$msg"* ]]; then ok "$name"; else bad "$name (rc=$rc: $out)"; fi
}

ENVF="$HOME/.config/veyron/veyron.env"
printf 'VEYRON_API_KEY=s3cret-api-key-value\nKUBECONFIG=/x/kubeconfig\n' >"$ENVF"
chmod 600 "$ENVF"
printf '#!/bin/sh\nexit 0\n' >"$T/veyron"; chmod +x "$T/veyron"
: >"$T/c.pem"; : >"$T/k.pem"

OUT="$("$SCRIPT" render --bin "$T/veyron" 2>&1)"
expect "render succeeds and names the label" '[[ "$OUT" == *"<key>Label</key><string>dev.zyvor.veyron</string>"* ]]'
expect "the API key never appears in the plist" '[[ "$OUT" != *s3cret-api-key-value* ]]'
expect "it sources the env file and execs api-serve" '[[ "$OUT" == *"exec "* && "$OUT" == *"api-serve"* && "$OUT" == *"veyron.env"* ]]'
# shellcheck disable=SC2034 # used inside the eval'd expectation below
BIND="--host '127.0.0.1' --port 5151"
expect "default bind is loopback:5151 over plain HTTP" '[[ "$OUT" == *"$BIND"* && "$OUT" != *"--tls"* ]]'
expect "paths with spaces survive" '[[ "$OUT" == *"home with space"* ]]'
expect "it is restarted" '[[ "$OUT" == *"<key>KeepAlive</key><true/>"* ]]'
if command -v plutil >/dev/null 2>&1; then
    echo "$OUT" >"$T/p.plist"
    expect "plutil accepts it" 'plutil -lint "$T/p.plist" >/dev/null'
fi

# The embedded shell command must itself parse, and must hand the env file's values to the program.
CMD="$(echo "$OUT" | sed -n '/<string>set -a/s/.*<string>\(.*\)<\/string>.*/\1/p' | sed 's/&quot;/"/g;s/&apos;/'"'"'/g;s/&amp;/\&/g;s/&lt;/</g;s/&gt;/>/g')"
# shellcheck disable=SC2034 # used inside the eval'd expectations below
RUN="$(echo "$CMD" | sed 's#exec .*#echo "$VEYRON_API_KEY"#')"
expect "the embedded sh command parses" 'sh -n -c "$CMD"'
expect "and exports the env file's values" '[ "$(sh -c "$RUN")" = s3cret-api-key-value ]'

# shellcheck disable=SC2034 # used inside the eval'd expectation below
T2="$("$SCRIPT" render --bin "$T/veyron" --host 0.0.0.0 --port 8443 --tls-cert "$T/c.pem" --tls-key "$T/k.pem" 2>&1)"
expect "--tls-cert/--tls-key/--host/--port reach the command line" '[[ "$T2" == *"--tls"* && "$T2" == *"8443"* && "$T2" == *"0.0.0.0"* && "$T2" == *"c.pem"* ]]'
refuses "a certificate without a key is refused" "must both be given" "$SCRIPT" render --bin "$T/veyron" --tls-cert "$T/c.pem"
refuses "a non-numeric port is refused" "--port must be a number" "$SCRIPT" render --bin "$T/veyron" --port 80x
refuses "a port out of range is refused" "1 to 65535" "$SCRIPT" render --bin "$T/veyron" --port 70000

chmod 644 "$ENVF"
refuses "a group/world-readable env file is refused" "chmod 600" "$SCRIPT" render --bin "$T/veyron"
chmod 600 "$ENVF"
grep -v '^KUBECONFIG=' "$ENVF" >"$ENVF.new" && mv "$ENVF.new" "$ENVF" && chmod 600 "$ENVF"
refuses "a missing required key is named" "does not set KUBECONFIG" "$SCRIPT" render --bin "$T/veyron"
echo "KUBECONFIG=/x/kubeconfig" >>"$ENVF"
refuses "a missing binary is refused" "not executable" "$SCRIPT" render --bin "$T/nope"
expect "an unknown command is refused" '! "$SCRIPT" frobnicate >/dev/null 2>&1'

echo
if [ "$FAILS" -eq 0 ]; then echo "all passed"; else echo "$FAILS failed"; exit 1; fi
