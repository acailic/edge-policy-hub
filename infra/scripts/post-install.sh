#!/bin/bash

set -euo pipefail

ACTION="${1:-configure}"
EDGE_USER="edge-policy"
EDGE_GROUP="edge-policy"
BASE_DIR="/var/lib/edge-policy-hub"
CONFIG_DIR="/etc/edge-policy-hub"
LOG_DIR="/var/log/edge-policy-hub"
DATA_BACKUP_DIR="${BASE_DIR}/backups/pre-${ACTION}-$(date +%Y%m%d-%H%M%S)"
HMAC_SECRET="${CONFIG_DIR}/hmac-secret"

log() {
  echo "[$(date --iso-8601=seconds)] $*"
}

ensure_group() {
  if ! getent group "${EDGE_GROUP}" >/dev/null; then
    groupadd --system "${EDGE_GROUP}"
  fi
}

ensure_user() {
  if ! id -u "${EDGE_USER}" >/dev/null 2>&1; then
    useradd --system --no-create-home --shell /bin/false --gid "${EDGE_GROUP}" "${EDGE_USER}"
  fi
}

create_directories() {
  install -d -m 0750 -o "${EDGE_USER}" -g "${EDGE_GROUP}" "${BASE_DIR}"
  install -d -m 0750 -o "${EDGE_USER}" -g "${EDGE_GROUP}" "${BASE_DIR}/config/tenants.d"
  install -d -m 0750 -o "${EDGE_USER}" -g "${EDGE_GROUP}" "${BASE_DIR}/data/audit"
  install -d -m 0750 -o "${EDGE_USER}" -g "${EDGE_GROUP}" "${BASE_DIR}/data/quota"
  install -d -m 0755 -o root -g root "${CONFIG_DIR}"
  install -d -m 0755 -o root -g root "${LOG_DIR}"
}

generate_secret() {
  if [[ ! -f "${HMAC_SECRET}" ]]; then
    umask 0177
    openssl rand -base64 32 > "${HMAC_SECRET}"
    chown "${EDGE_USER}:${EDGE_GROUP}" "${HMAC_SECRET}"
  fi
}

backup_data() {
  if [[ -d "${BASE_DIR}" ]]; then
    mkdir -p "$(dirname "${DATA_BACKUP_DIR}")"
    tar czf "${DATA_BACKUP_DIR}.tar.gz" -C "${BASE_DIR}" .
  fi
}

fresh_install() {
  log "Performing fresh Edge Policy Hub installation."
  ensure_group
  ensure_user
  create_directories
  generate_secret
  systemctl daemon-reload
  systemctl enable edge-policy-hub.target
  systemctl start edge-policy-hub.target || log "Services started with warnings."
  log "Installation complete."
}

upgrade_install() {
  log "Upgrading Edge Policy Hub."
  systemctl stop edge-policy-hub.target || true
  backup_data
  ensure_group
  ensure_user
  create_directories
  generate_secret
  systemctl daemon-reload
  systemctl start edge-policy-hub.target || log "Services restarted with warnings."
  log "Upgrade complete."
}

main() {
  case "${ACTION}" in
    configure|1)
      if [[ -f "${BASE_DIR}/.installed" ]]; then
        upgrade_install
      else
        fresh_install
        touch "${BASE_DIR}/.installed"
      fi
      ;;
    *)
      log "post-install script received unsupported action '${ACTION}', assuming configure."
      fresh_install
      ;;
  esac
  log "Post-installation tasks finished."
}

main "$@"
