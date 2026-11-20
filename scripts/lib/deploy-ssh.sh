#!/usr/bin/env bash
# shellcheck shell=bash
# Shared SSH options for remote deploy scripts (explicit port 22 by default).

[[ -n "${_DEPLOY_SSH_LOADED:-}" ]] && return 0
_DEPLOY_SSH_LOADED=1

DEPLOY_SSH_PORT="${DEPLOY_SSH_PORT:-22}"
DEPLOY_SSH_TIMEOUT="${DEPLOY_SSH_TIMEOUT:-20}"
DEPLOY_SSH_PASSWORD="${DEPLOY_SSH_PASSWORD:-}"

DEPLOY_SSH_OPTS=(
    -p "${DEPLOY_SSH_PORT}"
    -o ConnectTimeout="${DEPLOY_SSH_TIMEOUT}"
)

if [[ -n "${DEPLOY_SSH_PASSWORD}" ]]; then
    DEPLOY_SSH_OPTS+=(
        -o PreferredAuthentications=password
        -o PubkeyAuthentication=no
        -o StrictHostKeyChecking=accept-new
    )
fi

DEPLOY_SSH_BATCH_OPTS=(
    -p "${DEPLOY_SSH_PORT}"
    -o ConnectTimeout="${DEPLOY_SSH_TIMEOUT}"
    -o StrictHostKeyChecking=accept-new
)

if [[ -n "${DEPLOY_SSH_PASSWORD}" ]]; then
    DEPLOY_SSH_BATCH_OPTS+=(
        -o PreferredAuthentications=password
        -o PubkeyAuthentication=no
    )
else
    DEPLOY_SSH_BATCH_OPTS+=(-o BatchMode=yes)
fi

DEPLOY_SSH_STREAM_OPTS=(
    -p "${DEPLOY_SSH_PORT}"
    -o ServerAliveInterval=30
    -o ServerAliveCountMax=120
    -o TCPKeepAlive=yes
    -o ConnectTimeout="${DEPLOY_SSH_TIMEOUT}"
)

if [[ -n "${DEPLOY_SSH_PASSWORD}" ]]; then
    DEPLOY_SSH_STREAM_OPTS+=(
        -o PreferredAuthentications=password
        -o PubkeyAuthentication=no
        -o StrictHostKeyChecking=accept-new
    )
fi

deploy_rsync_ssh() {
    if [[ -n "${DEPLOY_SSH_PASSWORD}" ]] && command -v sshpass >/dev/null 2>&1; then
        printf 'sshpass -p %q ssh -p %s -o StrictHostKeyChecking=no -o ConnectTimeout=%s -o PreferredAuthentications=password -o PubkeyAuthentication=no' \
            "${DEPLOY_SSH_PASSWORD}" "${DEPLOY_SSH_PORT}" "${DEPLOY_SSH_TIMEOUT}"
    else
        printf 'ssh -p %s -o StrictHostKeyChecking=no -o ConnectTimeout=%s' \
            "${DEPLOY_SSH_PORT}" "${DEPLOY_SSH_TIMEOUT}"
    fi
}

deploy_ssh_preflight() {
    local remote="$1"
    local err=""
    if [[ -n "${DEPLOY_SSH_PASSWORD}" ]] && command -v sshpass >/dev/null 2>&1; then
        if err=$(sshpass -p "${DEPLOY_SSH_PASSWORD}" ssh "${DEPLOY_SSH_BATCH_OPTS[@]}" "${remote}" "true" 2>&1); then
            return 0
        fi
    elif err=$(ssh "${DEPLOY_SSH_BATCH_OPTS[@]}" "${remote}" "true" 2>&1); then
        return 0
    fi
    printf '%s\n' "${err}"
    return 1
}

deploy_ssh() {
    if [[ -n "${DEPLOY_SSH_PASSWORD}" ]] && command -v sshpass >/dev/null 2>&1; then
        sshpass -p "${DEPLOY_SSH_PASSWORD}" ssh "${DEPLOY_SSH_OPTS[@]}" "$@"
    else
        ssh "${DEPLOY_SSH_OPTS[@]}" "$@"
    fi
}

deploy_ssh_stream() {
    if [[ -n "${DEPLOY_SSH_PASSWORD}" ]] && command -v sshpass >/dev/null 2>&1; then
        sshpass -p "${DEPLOY_SSH_PASSWORD}" ssh "${DEPLOY_SSH_STREAM_OPTS[@]}" "$@"
    else
        ssh "${DEPLOY_SSH_STREAM_OPTS[@]}" "$@"
    fi
}
