#!/bin/bash

set -euo pipefail

EDGE_USER="edge-policy"
EDGE_GROUP="edge-policy"
BASE_DIR="/var/lib/edge-policy-hub"
CONFIG_DIR="${BASE_DIR}/config"
TENANTS_DIR="${CONFIG_DIR}/tenants.d"
AUDIT_DIR="${BASE_DIR}/data/audit"
QUOTA_DIR="${BASE_DIR}/data/quota"
LOG_DIR="/var/log/edge-policy-hub"
SYSTEMD_DIR="/etc/systemd/system"
BIN_TARGET="/usr/local/bin"
HMAC_SECRET="/etc/edge-policy-hub/hmac-secret"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RESOURCE_DIR="${SCRIPT_DIR}"
LOG_FILE="${LOG_DIR}/install.log"

required_cmds=("systemctl" "useradd" "groupadd" "openssl" "install" "cp")
service_files=(
  "edge-policy-enforcer.service"
  "edge-policy-proxy-http.service"
  "edge-policy-bridge-mqtt.service"
  "edge-policy-audit-store.service"
  "edge-policy-quota-tracker.service"
  "edge-policy-hub.target"
)
binaries=(
  "edge-policy-enforcer"
  "edge-policy-proxy-http"
  "edge-policy-bridge-mqtt"
  "edge-policy-audit-store"
  "edge-policy-quota-tracker"
)

log() {
  echo "[$(date --iso-8601=seconds)] $*"
}

abort() {
  log "ERROR: $*"
  exit 1
}

ensure_root() {
  if [[ "${EUID}" -ne 0 ]]; then
    abort "This installer must be run as root."
  fi
}

check_commands() {
  for cmd in "${required_cmds[@]}"; do
    if ! command -v "${cmd}" >/dev/null 2>&1; then
      abort "Required command '${cmd}' not found."
    fi
  done
}

prepare_logging() {
  mkdir -p "${LOG_DIR}"
  touch "${LOG_FILE}"
  chmod 0644 "${LOG_FILE}"
  exec > >(tee -a "${LOG_FILE}") 2>&1
}

ensure_group() {
  if ! getent group "${EDGE_GROUP}" >/dev/null; then
    log "Creating system group '${EDGE_GROUP}'."
    groupadd --system "${EDGE_GROUP}"
  fi
}

ensure_user() {
  if ! id -u "${EDGE_USER}" >/dev/null 2>&1; then
    log "Creating system user '${EDGE_USER}'."
    useradd --system --no-create-home --shell /bin/false --gid "${EDGE_GROUP}" "${EDGE_USER}"
  fi
}

create_directories() {
  log "Creating directory structure at ${BASE_DIR}."
  install -d -m 0750 -o "${EDGE_USER}" -g "${EDGE_GROUP}" "${BASE_DIR}"
  install -d -m 0750 -o "${EDGE_USER}" -g "${EDGE_GROUP}" "${CONFIG_DIR}"
  install -d -m 0750 -o "${EDGE_USER}" -g "${EDGE_GROUP}" "${TENANTS_DIR}"
  install -d -m 0750 -o "${EDGE_USER}" -g "${EDGE_GROUP}" "${BASE_DIR}/data"
  install -d -m 0750 -o "${EDGE_USER}" -g "${EDGE_GROUP}" "${AUDIT_DIR}"
  install -d -m 0750 -o "${EDGE_USER}" -g "${EDGE_GROUP}" "${QUOTA_DIR}"
  install -d -m 0755 -o root -g root /etc/edge-policy-hub
  install -d -m 0755 -o root -g root "${LOG_DIR}"
}

locate_binary() {
  local name="$1"
  local paths=(
    "${RESOURCE_DIR}/${name}"
    "${RESOURCE_DIR}/../${name}"
    "${SCRIPT_DIR}/../${name}"
    "${SCRIPT_DIR}/${name}"
  )
  for candidate in "${paths[@]}"; do
    if [[ -f "${candidate}" ]]; then
      echo "${candidate}"
      return 0
    fi
    if [[ -f "${candidate}.exe" ]]; then
      echo "${candidate}.exe"
      return 0
    fi
  done
  return 1
}

install_binaries() {
  log "Installing service binaries to ${BIN_TARGET}."
  for bin in "${binaries[@]}"; do
    if ! src="$(locate_binary "${bin}")"; then
      abort "Unable to locate binary '${bin}' near ${RESOURCE_DIR}."
    fi
    log "Copying ${src} -> ${BIN_TARGET}/${bin}"
    install -m 0755 "${src}" "${BIN_TARGET}/${bin}"
  done
}

install_services() {
  log "Installing systemd unit files."
  for svc in "${service_files[@]}"; do
    local source_path="${SCRIPT_DIR}/${svc}"
    if [[ ! -f "${source_path}" ]]; then
      abort "Missing service definition ${source_path}."
    fi
    install -m 0644 "${source_path}" "${SYSTEMD_DIR}/${svc}"
  done
}

generate_hmac_secret() {
  if [[ -f "${HMAC_SECRET}" ]]; then
    log "Existing HMAC secret detected at ${HMAC_SECRET}; preserving."
    return
  fi
  log "Generating HMAC secret."
  umask 0177
  openssl rand -base64 32 > "${HMAC_SECRET}"
  chown "${EDGE_USER}:${EDGE_GROUP}" "${HMAC_SECRET}"
}

enable_services() {
  log "Reloading systemd daemon."
  systemctl daemon-reload
  log "Enabling Edge Policy Hub services."
  systemctl enable edge-policy-hub.target
  log "Starting Edge Policy Hub services."
  systemctl start edge-policy-hub.target
}

verify_services() {
  log "Verifying service status."
  systemctl status --no-pager edge-policy-hub.target || true
}

main() {
  ensure_root
  check_commands
  prepare_logging
  log "Starting Edge Policy Hub systemd installation."
  ensure_group
  ensure_user
  create_directories
  install_binaries
  install_services
  generate_hmac_secret
  enable_services
  verify_services
  log "Edge Policy Hub installation complete."
  cat <<EOF

Edge Policy Hub services installed successfully.

Next steps:
  1. Launch the desktop UI to configure tenants and policies.
  2. Review logs with 'journalctl -u edge-policy-hub.target -f'.
  3. Place policy bundles in ${TENANTS_DIR}.

EOF
}

main "$@"
