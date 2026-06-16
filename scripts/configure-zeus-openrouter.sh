#!/usr/bin/env bash
# Wire Ask Zeus (Veyron Copilot LLM) to OpenRouter using keys from ~/.zshrc.
#
# Local API:
#   eval "$(./scripts/configure-zeus-openrouter.sh --print-env)"
#   cargo run --features web -- api-serve ...
#
# Remote cluster (default veyron-integrations secret + rollout restart):
#   ./scripts/configure-zeus-openrouter.sh HOST USER
#   ./scripts/configure-zeus-openrouter.sh <host> <user>

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=scripts/lib/read-openrouter-env.sh
source "$ROOT/scripts/lib/read-openrouter-env.sh"

NS="${VEYRON_NAMESPACE:-veyron-system}"
SECRET="${VMROGUE_INTEGRATIONS_SECRET:-veyron-integrations}"
RC_FILE="${VMROGUE_ZSHRC:-${HOME}/.zshrc}"

print_env() {
  read_openrouter_env "$RC_FILE"
}

apply_local_env() {
  while IFS= read -r line; do
    export "$line"
  done < <(print_env)
  echo "Zeus OpenRouter env loaded from $RC_FILE (mode=${VMROGUE_AI_MODE}, model=${VMROGUE_AI_MODEL})" >&2
}

remote_kubectl() {
  local host="$1" user="$2"
  shift 2
  ssh -o BatchMode=yes -o ConnectTimeout=20 "${user}@${host}" \
    "sudo /usr/local/bin/k3s kubectl -n ${NS} $*"
}

apply_remote() {
  local host="$1" user="$2"
  local tmp
  tmp="$(mktemp)"
  print_env >"$tmp"

  echo "Updating secret ${SECRET} in ${NS} on ${user}@${host}…" >&2
  remote_kubectl "$host" "$user" delete secret "$SECRET" --ignore-not-found
  scp -q -o BatchMode=yes "$tmp" "${user}@${host}:/tmp/veyron-zeus-openrouter.env"
  remote_kubectl "$host" "$user" create secret generic "$SECRET" \
    --from-env-file=/tmp/veyron-zeus-openrouter.env
  ssh -o BatchMode=yes "${user}@${host}" "rm -f /tmp/veyron-zeus-openrouter.env"
  rm -f "$tmp"
  remote_kubectl "$host" "$user" rollout restart deployment/veyron-api
  remote_kubectl "$host" "$user" rollout status deployment/veyron-api --timeout=180s
  echo "Done. Verify: curl -sk -H 'X-API-Key: …' https://${host}:30151/api/v1/experience/copilot/status" >&2
}

case "${1:-}" in
  --print-env)
    print_env
    ;;
  --local)
    apply_local_env
    ;;
  -h|--help)
    cat <<EOF
Usage:
  $0 --print-env          # emit VMROGUE_AI_* for eval
  $0 --local              # export into current shell
  $0 HOST USER            # patch ${SECRET} on remote k3s and restart API

Environment:
  VMROGUE_ZSHRC           rc file to read (default ~/.zshrc)
  VEYRON_NAMESPACE       Kubernetes namespace (default veyron-system)
EOF
    ;;
  "")
    apply_local_env
    ;;
  *)
    apply_remote "$1" "$2"
    ;;
esac
