#!/usr/bin/env bash
# Copyright 2026 Zyvor AI Labs · https://zyvor.dev
# SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0
# Read-only authenticated smoke checks; never starts or stops a VM.
set -euo pipefail
: "${VEYRON_URL:?Set VEYRON_URL to your API base URL}"
: "${VEYRON_API_KEY:?Set VEYRON_API_KEY}"
: "${VEYRON_NAMESPACE:?Set one explicit namespace}"
if [[ ! "$VEYRON_NAMESPACE" =~ ^[a-z0-9]([a-z0-9-]*[a-z0-9])?$ ]] || [[ "$VEYRON_NAMESPACE" == all ]] || (( ${#VEYRON_NAMESPACE} > 63 )); then
  echo "Use one DNS namespace" >&2
  exit 1
fi
for route in "/enterprise/capabilities" "/enterprise/operations?namespace=$VEYRON_NAMESPACE"; do
  curl --fail --silent --show-error --max-time 30 \
    --header "X-API-Key: $VEYRON_API_KEY" "${VEYRON_URL%/}/api/v1$route"
done
