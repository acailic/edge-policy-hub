#!/bin/bash

set -euo pipefail

ACTION="${1:-remove}"
LOG_FILE="/var/log/edge-policy-hub/uninstall.log"

log() {
  local message="$1"
  mkdir -p "$(dirname "${LOG_FILE}")"
  echo "[$(date --iso-8601=seconds)] $message" | tee -a "${LOG_FILE}"
}

stop_services() {
  log "Stopping Edge Policy Hub services."
  systemctl stop edge-policy-hub.target || true
  systemctl stop edge-policy-enforcer.service || true
  systemctl stop edge-policy-proxy-http.service || true
  systemctl stop edge-policy-bridge-mqtt.service || true
  systemctl stop edge-policy-audit-store.service || true
  systemctl stop edge-policy-quota-tracker.service || true
}

disable_services() {
  log "Disabling Edge Policy Hub services."
  systemctl disable edge-policy-hub.target || true
  systemctl disable edge-policy-enforcer.service || true
  systemctl disable edge-policy-proxy-http.service || true
  systemctl disable edge-policy-bridge-mqtt.service || true
  systemctl disable edge-policy-audit-store.service || true
  systemctl disable edge-policy-quota-tracker.service || true
}

main() {
  case "${ACTION}" in
    remove|0)
      log "Pre-removal hook triggered."
      stop_services
      disable_services
      ;;
    upgrade|1)
      log "Pre-upgrade hook triggered."
      stop_services
      ;;
    *)
      log "Pre-remove invoked with argument '${ACTION}'. Performing safe stop."
      stop_services
      ;;
  esac
  log "Pre-removal tasks complete."
}

main "$@"
