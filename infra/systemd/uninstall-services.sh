#!/bin/bash

set -euo pipefail

BASE_DIR="/var/lib/edge-policy-hub"
CONFIG_DIR="/etc/edge-policy-hub"
LOG_DIR="/var/log/edge-policy-hub"
SYSTEMD_DIR="/etc/systemd/system"
BIN_TARGET="/usr/local/bin"
EDGE_USER="edge-policy"
EDGE_GROUP="edge-policy"
LOG_FILE="/tmp/edge-policy-hub-uninstall.log"

KEEP_DATA="prompt"
REMOVE_ALL=false
DRY_RUN=false
DO_BACKUP=false

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

usage() {
  cat <<EOF
Usage: $(basename "$0") [options]

Options:
  --keep-data      Preserve data and configuration directories.
  --remove-all     Remove all data, configuration, and logs without prompt.
  --dry-run        Show actions without executing them.
  --backup         Create backup archive before removing data.
  -h, --help       Show this help message.
EOF
}

log() {
  echo "[$(date --iso-8601=seconds)] $*"
}

prompt_yes_no() {
  local question="$1"
  local default="${2:-N}"
  local response
  read -r -p "${question} " response
  response="${response:-${default}}"
  case "${response}" in
    [Yy]*) return 0 ;;
    *) return 1 ;;
  esac
}

run() {
  if ${DRY_RUN}; then
    log "[DRY-RUN] $*"
  else
    log "$*"
    eval "$@"
  fi
}

ensure_root() {
  if [[ "${EUID}" -ne 0 ]]; then
    echo "This uninstaller must be run as root." >&2
    exit 1
  fi
}

parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --keep-data)
        KEEP_DATA=true
        shift
        ;;
      --remove-all)
        REMOVE_ALL=true
        KEEP_DATA=false
        shift
        ;;
      --dry-run)
        DRY_RUN=true
        shift
        ;;
      --backup)
        DO_BACKUP=true
        shift
        ;;
      -h|--help)
        usage
        exit 0
        ;;
      *)
        echo "Unknown option: $1" >&2
        usage
        exit 1
        ;;
    esac
  done

  if ${REMOVE_ALL} && [[ "${KEEP_DATA}" == true ]]; then
    echo "Options --remove-all and --keep-data are mutually exclusive." >&2
    exit 1
  fi
}

prepare_logging() {
  touch "${LOG_FILE}"
  chmod 0644 "${LOG_FILE}"
  exec > >(tee -a "${LOG_FILE}") 2>&1
}

stop_services() {
  run "systemctl stop edge-policy-hub.target || true"
  run "systemctl disable edge-policy-hub.target || true"
}

remove_services() {
  for svc in "${service_files[@]}"; do
    if [[ -f "${SYSTEMD_DIR}/${svc}" ]]; then
      run "rm -f '${SYSTEMD_DIR}/${svc}'"
    fi
  done
  run "systemctl daemon-reload"
}

remove_binaries() {
  for bin in "${binaries[@]}"; do
    if [[ -f "${BIN_TARGET}/${bin}" ]]; then
      run "rm -f '${BIN_TARGET}/${bin}'"
    fi
  done
}

backup_data() {
  local timestamp
  timestamp="$(date +%Y%m%d-%H%M%S)"
  local archive="/tmp/edge-policy-hub-backup-${timestamp}.tar.gz"
  if ${DRY_RUN}; then
    log "[DRY-RUN] Would create backup archive ${archive}"
    return
  fi
  log "Creating backup at ${archive}"
  tar czf "${archive}" --ignore-failed-read \
    "${BASE_DIR}" \
    "${CONFIG_DIR}" \
    "${LOG_DIR}" || log "Backup completed with warnings."
}

handle_data_removal() {
  if [[ "${KEEP_DATA}" == true ]]; then
    log "Preserving data and configuration as requested."
    return
  fi

  if ! ${REMOVE_ALL}; then
    if [[ "${KEEP_DATA}" == "prompt" ]]; then
      if prompt_yes_no "Do you want to keep data directories? (y/N)" "N"; then
        KEEP_DATA=true
        log "Data retention selected."
        return
      fi
    fi

    if ! prompt_yes_no "This will delete all Edge Policy Hub data. Continue? (y/N)" "N"; then
      log "Data preservation selected."
      return
    fi
  fi

  if ${DO_BACKUP}; then
    backup_data
  fi

  for path in "${BASE_DIR}" "${CONFIG_DIR}" "${LOG_DIR}"; do
    if [[ -e "${path}" ]]; then
      run "rm -rf '${path}'"
    fi
  done
}

remove_user_group() {
  if prompt_yes_no "Remove '${EDGE_USER}' system user? (y/N)" "N"; then
    if id -u "${EDGE_USER}" >/dev/null 2>&1; then
      run "userdel '${EDGE_USER}'"
    fi
    if getent group "${EDGE_GROUP}" >/dev/null 2>&1; then
      run "groupdel '${EDGE_GROUP}'"
    fi
  else
    log "Preserving system user and group."
  fi
}

print_summary() {
  log "Edge Policy Hub uninstallation completed."
  if [[ "${KEEP_DATA}" == true ]]; then
    log "Data preserved at ${BASE_DIR} and ${CONFIG_DIR}."
  else
    log "Data removal processed."
  fi
  log "Detailed log: ${LOG_FILE}"
}

main() {
  ensure_root
  parse_args "$@"
  prepare_logging
  log "Starting Edge Policy Hub uninstallation."
  stop_services
  remove_services
  remove_binaries
  handle_data_removal
  remove_user_group
  print_summary
}

main "$@"
