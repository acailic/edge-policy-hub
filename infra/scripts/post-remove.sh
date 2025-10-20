#!/bin/bash

set -euo pipefail

ACTION="${1:-remove}"
BASE_DIR="/var/lib/edge-policy-hub"
CONFIG_DIR="/etc/edge-policy-hub"
LOG_DIR="/var/log/edge-policy-hub"
EDGE_USER="edge-policy"
EDGE_GROUP="edge-policy"

log() {
  echo "[$(date --iso-8601=seconds)] $*"
}

purge_data() {
  log "Removing data, configuration, and logs."
  rm -rf "${BASE_DIR}" "${CONFIG_DIR}" "${LOG_DIR}"
  if id -u "${EDGE_USER}" >/dev/null 2>&1; then
    userdel "${EDGE_USER}" || true
  fi
  if getent group "${EDGE_GROUP}" >/dev/null 2>&1; then
    groupdel "${EDGE_GROUP}" || true
  fi
}

remove_units() {
  rm -f /etc/systemd/system/edge-policy-*.service
  rm -f /etc/systemd/system/edge-policy-hub.target
  systemctl daemon-reload || true
}

main() {
  remove_units
  case "${ACTION}" in
    purge|0)
      purge_data
      log "Edge Policy Hub fully removed."
      ;;
    remove|upgrade|1)
      log "Edge Policy Hub binaries removed. Data preserved at ${BASE_DIR}."
      ;;
    *)
      log "post-remove invoked with argument '${ACTION}'. No destructive actions taken."
      ;;
  esac
}

main "$@"
