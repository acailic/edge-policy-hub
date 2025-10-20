#!/bin/bash

set -euo pipefail

SKIP_DOCKER=false
SKIP_TAURI=false
PLATFORM="all"
OVERRIDE_VERSION=""

usage() {
  cat <<EOF
Usage: $(basename "$0") [options]

Options:
  --skip-docker         Skip building Docker images.
  --skip-tauri          Skip building Tauri installers.
  --platform <target>   Limit build to linux|windows|macos (default: all).
  --version <version>   Override version for artifact naming.
  -h, --help            Show this help message.
EOF
}

parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --skip-docker)
        SKIP_DOCKER=true
        shift
        ;;
      --skip-tauri)
        SKIP_TAURI=true
        shift
        ;;
      --platform)
        PLATFORM="$2"
        shift 2
        ;;
      --version)
        OVERRIDE_VERSION="$2"
        shift 2
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
}

ensure_commands() {
  local required=("rustc" "cargo")
  if ! ${SKIP_TAURI}; then
    required+=("node" "pnpm")
  fi
  if ! ${SKIP_DOCKER}; then
    required+=("docker" "docker-compose")
  fi
  for cmd in "${required[@]}"; do
    if ! command -v "${cmd}" >/dev/null 2>&1; then
      echo "Required command '${cmd}' not found in PATH." >&2
      exit 1
    fi
  done
}

determine_version() {
  if [[ -n "${OVERRIDE_VERSION}" ]]; then
    VERSION="${OVERRIDE_VERSION}"
    return
  fi
  if command -v cargo >/dev/null 2>&1 && command -v jq >/dev/null 2>&1; then
    VERSION="$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.name == "edge-policy-tauri-ui") | .version')"
  fi
  VERSION="${VERSION:-0.0.0}"
}

build_backend() {
  echo "Building backend services in release mode."
  cargo build --release --workspace
  find target/release -maxdepth 1 -type f -name "edge-policy-*" -print0 | xargs -0 strip || true
}

build_tauri() {
  if ${SKIP_TAURI}; then
    echo "Skipping Tauri build (per flag)."
    return
  fi
  pushd apps/tauri-ui >/dev/null
  pnpm install
  case "${PLATFORM}" in
    linux)
      pnpm tauri build --target linux
      ;;
    windows)
      pnpm tauri build --target windows
      ;;
    macos)
      pnpm tauri build --target macos
      ;;
    *)
      pnpm tauri build
      ;;
  esac
  popd >/dev/null
}

build_docker() {
  if ${SKIP_DOCKER}; then
    echo "Skipping Docker image build (per flag)."
    return
  fi
  docker-compose -f infra/docker/docker-compose.yml build
}

prepare_dist() {
  DIST_DIR="dist"
  rm -rf "${DIST_DIR}"
  mkdir -p "${DIST_DIR}"
}

collect_installers() {
  if ${SKIP_TAURI}; then
    return
  fi
  local bundle_dir="apps/tauri-ui/src-tauri/target/release/bundle"
  if [[ -d "${bundle_dir}" ]]; then
    echo "Collecting Tauri installer artifacts."
    find "${bundle_dir}" -type f \( -name "*.deb" -o -name "*.rpm" -o -name "*.AppImage" -o -name "*.exe" -o -name "*.msi" -o -name "*.dmg" -o -name "*.app.tar.gz" \) \
      -exec cp {} "${DIST_DIR}/" \;
  fi
}

export_docker_images() {
  if ${SKIP_DOCKER}; then
    return
  fi
  echo "Exporting Docker images."
  local services=("enforcer" "proxy-http" "bridge-mqtt" "audit-store" "quota-tracker")
  for svc in "${services[@]}"; do
    local image="ghcr.io/acailic/edge-policy-${svc}:latest"
    local archive="${DIST_DIR}/edge-policy-${svc}-docker.tar.gz"
    docker save "${image}" | gzip > "${archive}"
  done
}

generate_checksums() {
  if command -v sha256sum >/dev/null 2>&1; then
    (cd "${DIST_DIR}" && sha256sum * > SHA256SUMS)
  elif command -v shasum >/dev/null 2>&1; then
    (cd "${DIST_DIR}" && shasum -a 256 * > SHA256SUMS)
  else
    echo "Skipping checksum generation (sha256sum/shasum not available)."
  fi
}

print_summary() {
  echo ""
  echo "Build complete. Artifacts available in ${DIST_DIR}/"
  ls -lh "${DIST_DIR}"
  echo ""
  echo "Suggested next steps:"
  echo "  - Review SHA256SUMS and upload installers to release draft."
  if ! ${SKIP_DOCKER}; then
    echo "  - Load Docker images with 'docker load < artifact.tar.gz' on target hosts."
  fi
}

main() {
  parse_args "$@"
  ensure_commands
  determine_version
  build_backend
  build_tauri
  build_docker
  prepare_dist
  collect_installers
  export_docker_images
  generate_checksums
  print_summary
}

main "$@"
