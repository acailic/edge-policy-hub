# Testing Guide

Edge Policy Hub uses a multi-layered testing strategy that spans unit, integration, end-to-end workflows, performance benchmarks, and UI automation. This guide explains how to run the suites, what each covers, and quality expectations.

## Overview

1. **Unit Tests** – Validate individual functions/modules (`#[test]`, Rego `test_` rules)
2. **Integration Tests** – Service-level API integration (`tests/` per service)
3. **End-to-End Tests** – Full stack orchestration (`tests/e2e/` harness)
4. **Performance Benchmarks** – Criterion.rs latency and throughput
5. **UI Tests** – Tauri desktop workflows via WebDriver or Playwright

## Running Tests

```bash
# Rust unit + integration tests
cargo test --workspace

# OPA/Rego policy tests
opa test libs/rego-bundles/policies/

# End-to-end suite (requires --features e2e-tests)
cargo test --test e2e --features e2e-tests

# MQTT E2E tests (opt-in via feature flag)
cargo test --test e2e --features mqtt-e2e -- --test-threads=1

# Benchmarks
cargo bench

# UI end-to-end tests (requires Playwright or WebDriver)
cd apps/tauri-ui
pnpm test:e2e
```

### Targeted Suites

```bash
# Enforcer service only
cargo test --package edge-policy-enforcer

# HTTP proxy E2E scenarios
cargo test --test e2e http_proxy --features e2e-tests

# Multi-tenant isolation checks
cargo test --test e2e multi_tenant --features e2e-tests

# Policy latency benchmarks
cargo bench --bench policy_latency
```

## Unit Tests

### Rust
- Located beside implementation in `src/` under `#[cfg(test)]` modules.
- Use mocks for external dependencies.
- Examples: policy-dsl parser, enforcer tenant validator, quota tracker persistence.

### Rego
- Reside in `libs/rego-bundles/policies/tests/`.
- Use `with` blocks for test data.
- Run via `opa test`.

## Integration Tests

- Found in `services/*/tests/`.
- Boot service with test configuration (random ports, temp directories).
- Use `reqwest::Client` and `wiremock` for HTTP assertions.
- Examples: enforcer bundle reload, proxy HTTP pipeline, MQTT bridge hooks.

## End-to-End Tests

### Harness (`tests/e2e/harness.rs`)
- Starts enforcer, proxy-http, bridge-mqtt, audit-store, quota-tracker via `cargo run`.
- Allocates random ports; isolates data in `tempfile::TempDir`.
- Provides helpers: `create_test_tenant`, `deploy_test_policy`, `wait_for_service_health`.

### Suites
- **http_proxy_tests.rs** – Policy enforcement, field redaction, quotas, cross-tenant blocking.
- **mqtt_bridge_tests.rs** – Topic namespace validation, quota enforcement, payload redaction.
- **multi_tenant_isolation_tests.rs** – Verifies isolation across policies, audit logs, quotas, MQTT sessions.
- **offline_first_tests.rs** – Simulates network partitions, ensures cached decisions and retries.
- **ui_workflow_tests.rs** – Tauri UI flows (tenant creation, policy compile/deploy, monitoring dashboards).

E2E tests require `--features e2e-tests` to avoid running in CI by default.

## Performance Benchmarks

### Criterion Suites
- `benches/policy_latency.rs` – Measures policy evaluation scenarios (simple, complex, concurrent). Targets: `p99 < 2ms`.
- `benches/end_to_end_latency.rs` – Measures HTTP proxy and MQTT end-to-end latency. Targets: `< 15ms p99`.

### Running Benchmarks

```bash
# All benchmarks
cargo bench

# Save baseline for regression detection
cargo bench --bench policy_latency -- --save-baseline main

# Compare against baseline
cargo bench --bench policy_latency -- --baseline main
```

Bench output includes percentile statistics and HTML reports (`target/criterion/**/report/index.html`).

## Test Data Management

- Use `tempfile::TempDir` for ephemeral data.
- Fixtures located in `examples/tenants/` and `examples/policies/`.
- Clean up resources in `Drop` or explicit `cleanup()` helpers.

## Continuous Integration

- GitHub Actions runs:
  - Unit + integration tests (`cargo test --workspace`)
  - OPA tests (`opa test ...`)
  - End-to-end tests (`cargo test --test e2e --features e2e-tests -- --test-threads=1`)
  - Benchmarks on main branch (`cargo bench -- --save-baseline ci`)
  - Coverage via Tarpaulin (`cargo tarpaulin --workspace`)
- CI fails if tests fail, coverage drops < 70%, or benchmark regression > 10%.

## Troubleshooting

- **Flaky E2E tests:** Increase health check timeouts, ensure ports free, inspect logs under `target/debug`.
- **Benchmark variance:** Close background apps, disable TurboBoost, increase sample size.
- **MQTT tests failing:** Confirm Mosquitto or other brokers not occupying ports 1883/1884.
- **UI automation:** Ensure WebDriver (e.g., geckodriver) on PATH; grant screen recording on macOS.

## Test Quality Standards

- Deterministic assertions with clear failure messages.
- Descriptive test names (e.g., `test_http_proxy_enforces_eu_data_residency`).
- Keep tests under ~2 seconds when possible; mark long tests with `#[ignore]`.
- Cover error paths (invalid tenants, quota exceeded, service outages).

## Coverage Goals

- **Code coverage:** 80% for enforcement, quota, and audit paths (`cargo tarpaulin --out Html`).
- **Scenario coverage:** All policy operators exercised; multi-tenant isolation verified; offline-first flows tested.

## Adding New Tests

1. Create or update unit tests for new modules.
2. Add integration tests if service behaviour changes.
3. Extend E2E harness for new workflows.
4. Update benchmarks when performance requirements change.
5. Document new commands in this guide when relevant.

Refer to `CONTRIBUTING.md` for PR requirements and `README.md` for quickstart instructions.
