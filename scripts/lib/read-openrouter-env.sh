#!/usr/bin/env bash
# Library: read_openrouter_env [rc-file]
# Prints VMROGUE_AI_* KEY=value lines (no logging of secrets).

read_openrouter_env() {
  local RC_FILE="${1:-${HOME}/.zshrc}"

  if [[ ! -f "$RC_FILE" ]]; then
    echo "read_openrouter_env: missing $RC_FILE" >&2
    return 1
  fi

  read_export() {
    local name="$1"
    local line val
    line="$(grep -E "^(#)?[[:space:]]*export[[:space:]]+${name}=" "$RC_FILE" 2>/dev/null | tail -1 || true)"
    [[ -n "$line" ]] || return 1
    line="${line#\#}"
    line="${line#export }"
    val="${line#${name}=}"
    val="${val%\"}"
    val="${val#\"}"
    val="${val%\'}"
    val="${val#\'}"
    printf '%s' "$val"
  }

  local API_KEY BASE_URL MODEL MODE APP_TITLE
  API_KEY="$(read_export ANTHROPIC_AUTH_TOKEN 2>/dev/null || true)"
  if [[ -z "$API_KEY" ]]; then
    API_KEY="$(read_export OPENROUTER_API_KEY 2>/dev/null || true)"
  fi
  if [[ -z "$API_KEY" ]]; then
    API_KEY="$(read_export VMROGUE_AI_API_KEY 2>/dev/null || true)"
  fi
  if [[ -z "$API_KEY" ]]; then
    echo "read_openrouter_env: no OpenRouter key in $RC_FILE" >&2
    return 1
  fi

  BASE_URL="$(read_export ANTHROPIC_BASE_URL 2>/dev/null || true)"
  if [[ -z "$BASE_URL" ]]; then
    BASE_URL="$(read_export OPENROUTER_API_URL 2>/dev/null || true)"
  fi
  if [[ -z "$BASE_URL" ]]; then
    BASE_URL="$(read_export VMROGUE_AI_URL 2>/dev/null || true)"
  fi
  if [[ -z "$BASE_URL" ]]; then
    BASE_URL="https://openrouter.ai/api/v1"
  elif [[ "$BASE_URL" == *openrouter.ai/api* && "$BASE_URL" != */v1 ]]; then
    BASE_URL="${BASE_URL%/}/v1"
  fi

  MODEL="$(read_export VMROGUE_AI_MODEL 2>/dev/null || true)"
  if [[ -z "$MODEL" ]]; then
    MODEL="$(read_export ANTHROPIC_MODEL 2>/dev/null || true)"
  fi
  if [[ -z "$MODEL" ]]; then
    MODEL="$(read_export OPENROUTER_MODEL 2>/dev/null || true)"
  fi
  if [[ -z "$MODEL" ]]; then
    MODEL="openrouter/free"
  fi

  MODE="$(read_export VMROGUE_AI_MODE 2>/dev/null || true)"
  if [[ -z "$MODE" ]]; then
    MODE="routing"
  fi

  APP_TITLE="$(read_export VMROGUE_AI_APP_TITLE 2>/dev/null || true)"
  if [[ -z "$APP_TITLE" ]]; then
    APP_TITLE="ZeusOS"
  fi

  printf 'VMROGUE_AI_API_KEY=%s\n' "$API_KEY"
  printf 'VMROGUE_AI_URL=%s\n' "$BASE_URL"
  printf 'VMROGUE_AI_MODEL=%s\n' "$MODEL"
  printf 'VMROGUE_AI_MODE=%s\n' "$MODE"
  printf 'VMROGUE_AI_APP_TITLE=%s\n' "$APP_TITLE"
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  read_openrouter_env "${1:-${HOME}/.zshrc}"
fi
