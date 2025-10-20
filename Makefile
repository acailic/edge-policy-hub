.PHONY: build test lint fmt clean dev ci install-tools \
	build-release build-tauri build-docker build-all \
	install-systemd uninstall-systemd docker-up docker-down docker-logs \
	generate-signing-keys help test-e2e test-integration test-opa \
	bench bench-policy bench-e2e test-all coverage verify all

build:
	cargo build --workspace

test:
	cargo test --workspace

test-integration:
	cargo test --workspace --tests

test-e2e:
	cargo test --test e2e --features e2e-tests -- --test-threads=1

test-opa:
	opa test libs/rego-bundles/policies/

lint:
	cargo clippy --workspace --all-features -- -D warnings
	cargo fmt --all -- --check

fmt:
	cargo fmt --all

clean:
	cargo clean

dev:
	cargo watch -x 'run --workspace'

ci:
	cargo build --workspace --all-features
	cargo test --workspace --all-features
	cargo clippy --workspace --all-features -- -D warnings
	cargo fmt --all -- --check

bench:
	cargo bench --workspace

bench-policy:
	cargo bench --bench policy_latency -- --save-baseline main

bench-e2e:
	cargo bench --bench end_to_end_latency -- --save-baseline main

test-all: test test-integration test-e2e test-opa

coverage:
	cargo tarpaulin --workspace --out Html --output-dir coverage/

verify: test-all bench coverage
	@echo "All verification steps completed successfully."

all: verify

install-tools:
	cargo install cargo-watch --locked
	cargo install cargo-audit --locked

build-release:
	cargo build --release --workspace
	find target/release -maxdepth 1 -type f -name 'edge-policy-*' -exec strip {} + || true

build-tauri:
	cd apps/tauri-ui && pnpm install && pnpm tauri build && \
		echo "Tauri installers available under src-tauri/target/release/bundle"

build-docker:
	docker-compose -f infra/docker/docker-compose.yml build

build-all: build-release build-tauri build-docker
	rm -rf dist
	mkdir -p dist
	find apps/tauri-ui/src-tauri/target/release/bundle -type f -exec cp {} dist/ \; 2>/dev/null || true
	for image in enforcer proxy-http bridge-mqtt audit-store quota-tracker; do \
		docker save ghcr.io/acailic/edge-policy-$$image:latest | gzip > dist/edge-policy-$$image-docker.tar.gz ; \
	done
	cd dist && { command -v sha256sum >/dev/null && sha256sum * > SHA256SUMS || shasum -a 256 * > SHA256SUMS; }

install-systemd:
	sudo infra/systemd/install-services.sh

uninstall-systemd:
	sudo infra/systemd/uninstall-services.sh

docker-up:
	docker-compose -f infra/docker/docker-compose.yml up -d

docker-down:
	docker-compose -f infra/docker/docker-compose.yml down

docker-logs:
	docker-compose -f infra/docker/docker-compose.yml logs -f

generate-signing-keys:
	scripts/generate-signing-keys.sh

help:
	@echo "Available targets:"
	@echo "  build                 - Debug build of all Rust crates"
	@echo "  test                  - Run test suite"
	@echo "  test-integration      - Run Rust integration tests across services"
	@echo "  test-e2e              - Execute full-stack end-to-end tests (requires services)"
	@echo "  test-opa              - Run Rego/OPA unit tests"
	@echo "  test-all              - Run unit, integration, E2E, and OPA tests"
	@echo "  bench                 - Execute all Criterion benchmarks"
	@echo "  bench-policy          - Benchmark policy evaluation latency (saves baseline 'main')"
	@echo "  bench-e2e             - Benchmark HTTP and MQTT end-to-end latency"
	@echo "  coverage              - Generate HTML code coverage with tarpaulin"
	@echo "  verify                - Run test-all, bench, and coverage (release gating pipeline)"
	@echo "  lint                  - Run clippy and fmt checks"
	@echo "  build-release         - Build backend binaries in release mode"
	@echo "  build-tauri           - Build desktop installers"
	@echo "  build-docker          - Build Docker images"
	@echo "  build-all             - Build binaries, installers, Docker images, and bundle artifacts"
	@echo "  install-systemd       - Install systemd services (requires sudo)"
	@echo "  uninstall-systemd     - Remove systemd services (requires sudo)"
	@echo "  docker-up             - Start Docker Compose deployment"
	@echo "  docker-down           - Stop Docker Compose deployment"
	@echo "  docker-logs           - Tail Docker Compose logs"
	@echo "  generate-signing-keys - Generate Tauri updater signing keys"
