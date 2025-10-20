#!/bin/bash

set -euo pipefail

KEY_DIR="${HOME}/.tauri"
KEY_PATH="${KEY_DIR}/edge-policy-hub.key"
CONFIG_PATH="apps/tauri-ui/src-tauri/tauri.conf.json"

log() {
  echo "[$(date --iso-8601=seconds)] $*"
}

ensure_tauri_cli() {
  if ! command -v tauri >/dev/null 2>&1; then
    log "Installing Tauri CLI globally."
    npm install -g @tauri-apps/cli
  fi
}

generate_keys() {
  mkdir -p "${KEY_DIR}"
  if [[ -f "${KEY_PATH}" ]]; then
    log "Existing key detected at ${KEY_PATH}. Move or remove it before generating a new key."
    exit 1
  fi
  log "Generating Tauri updater signing key."
  local output
  output="$(tauri signer generate -w "${KEY_PATH}")"
  echo "${output}"
  PUBLIC_KEY="$(grep -Eo '-----BEGIN PUBLIC KEY-----.*-----END PUBLIC KEY-----' <<<"${output}" | tr -d '\n' | sed 's/-----BEGIN PUBLIC KEY-----//;s/-----END PUBLIC KEY-----//')"
  if [[ -z "${PUBLIC_KEY:-}" ]]; then
    log "Warning: Unable to automatically parse public key from output."
  fi
}

update_config() {
  if [[ -z "${PUBLIC_KEY:-}" ]]; then
    return
  fi
  if ! command -v jq >/dev/null 2>&1; then
    log "jq not available; skipping automatic config update."
    return
  fi
  if [[ ! -f "${CONFIG_PATH}" ]]; then
    log "Tauri config not found at ${CONFIG_PATH}; skipping automatic update."
    return
  fi
  log "Updating updater public key in ${CONFIG_PATH}."
  tmp="$(mktemp)"
  jq --arg key "${PUBLIC_KEY}" '.updater.pubkey = $key' "${CONFIG_PATH}" > "${tmp}"
  mv "${tmp}" "${CONFIG_PATH}"
}

print_instructions() {
  cat <<EOF

Tauri updater signing key generated.

Private key: ${KEY_PATH}
Public key : ${PUBLIC_KEY:-<review command output>}

Next steps:
  1. Store the private key securely (for example, as TAURI_PRIVATE_KEY in repository secrets).
  2. Ensure the public key is configured in ${CONFIG_PATH} under updater.pubkey.
  3. Never commit the private key to version control.

EOF
}

main() {
  ensure_tauri_cli
  generate_keys
  update_config
  print_instructions
}

main "$@"
