# Edge Policy Hub – Tauri UI

This application provides a desktop experience for authoring and operating ABAC policies across Edge Policy Hub services.

## Features

- Monaco-powered Policy Builder with custom DSL syntax highlighting and inline error decorations.
- Real-time compilation against the `edge-policy-dsl` crate with detailed diagnostics.
- Integrated Test Simulator to exercise policies against live enforcer decisions.
- Version history browser with activation and rollback controls backed by the audit-store service.
- One-click deployment flow that persists bundles, writes Rego to the enforcer bundle directory, and triggers hot-reload.
- Monitoring dashboard with live decision streaming, quota visualisation, audit analytics, and desktop notifications.

## Self-Update Mechanism

- The desktop application checks GitHub releases on startup and every six hours.
- When a newer version is detected, an in-app banner surfaces the update with release notes.
- Operators can install immediately, defer, or skip a specific version.
- Downloads occur over HTTPS and signatures are validated against the public key embedded in `src-tauri/tauri.conf.json`.
- Manual checks are available in **Settings → About → Check for Updates**.

### Update Sequence

1. Fetch `https://github.com/acailic/edge-policy-hub/releases/latest/download/latest.json`.
2. Compare the embedded version with the running build.
3. Download the platform-specific installer and verify the signature.
4. Install in-place and relaunch via the Tauri updater plugin.

## Policy Management Workflow

1. Open *Tenants → Policies* for the tenant you want to manage.
2. Author the policy in the DSL editor; templates help you start quickly.
3. Press **Compile** (Cmd/Ctrl + S) to generate Rego and surface any errors.
4. Switch to **Test Simulator** to validate behaviour with ABAC inputs.
5. Deploy as **Draft** for offline review or **Deploy & Activate** for immediate enforcement.
6. Review version history, compare bundles, and rollback if necessary.

## DSL Syntax Reference

- Rules follow the form `allow|deny <action> <resource_type> if <conditions>`.
- Attribute namespaces: `subject.*`, `resource.*`, `environment.*`, `action`.
- Supported operators: `==`, `!=`, `<`, `>`, `<=`, `>=`, logical `and`, `or`, `not`, and `in`.
- See `docs/policy-dsl.md` and `libs/policy-dsl/examples` for complete examples.

## Keyboard Shortcuts

- **Cmd/Ctrl + S** – Compile current policy.
- **Cmd/Ctrl + Shift + D** – Open deployment dialog.
- **Cmd/Ctrl + T** – Toggle the Test Simulator panel.

## Integration

- Compilation uses the local workspace dependency on `libs/policy-dsl`, ensuring parity with backend validation.
- Deployment writes Rego bundles into `config/tenants.d/{tenant_id}` and invokes the enforcer reload endpoint for instant updates.
- Version management leverages the audit-store REST API, mirroring the CLI and automation tooling.
- The monitoring dashboard connects to the enforcer WebSocket feed (`/v1/stream/decisions`), polls audit-store for logs, and tracks quota usage via quota-tracker APIs.

## Monitoring Workflow

1. Open **Monitoring** for a global overview or navigate to **Tenants → Monitor** for a scoped tenant dashboard.
2. Observe the real-time decision stream; pause, filter, or clear the feed as required.
3. Review quota gauges for message and bandwidth usage, with thresholds and notifications surfacing automatically.
4. Search, sort, and export audit logs directly from the dashboard for incident review.
5. Respond to desktop notifications that alert you to quota warnings, exceedances, or denied actions.

More details, including architecture diagrams and troubleshooting steps, live in [`docs/monitoring-dashboard.md`](../../docs/monitoring-dashboard.md).

## WebSocket Connection

- The UI dials `ws://127.0.0.1:8181/v1/stream/decisions` with optional `tenant_id` query parameter filtering.
- Connection status is surfaced in the UI (Connected / Connecting / Disconnected) and retries follow an exponential backoff strategy capped at 10 seconds.
- Decision events mirror the backend `DecisionEvent` struct, letting the UI display latency, redact fields, and contextual metadata.

## Desktop Notifications

- Initialisation occurs through `tauri-plugin-notification`, with permissions configured in `src-tauri/capabilities/default.json`.
- The `useNotifications` hook checks permission state, requests once if needed, and exposes helpers for quota warnings and policy violations.
- Notifications are debounced (5 minutes for quota alerts, 10 seconds for violation alerts) to prevent operator fatigue.
- Users can disable OS notifications to silence alerts while retaining in-app visibility.

## Building Installers

### Prerequisites

- Backend binaries built in release mode: `cargo build --release --workspace`
- Node.js 18+ and pnpm installed locally
- Platform-specific dependencies (e.g., `libwebkit2gtk` on Linux)

### Build Command

```bash
pnpm tauri build
```

### Generated Artifacts

- Linux: `src-tauri/target/release/bundle/deb/edge-policy-hub_<version>_amd64.deb`
- Linux: `src-tauri/target/release/bundle/rpm/edge-policy-hub-<version>-1.x86_64.rpm`
- Windows: `src-tauri/target/release/bundle/msi/Edge Policy Hub_<version>_x64_en-US.msi`
- macOS: `src-tauri/target/release/bundle/dmg/Edge Policy Hub_<version>_x64.dmg`

### Platform Notes

- Linux: install `libwebkit2gtk-4.1-dev`, `libssl-dev`, and `patchelf`.
- Windows: ensure Microsoft WebView2 runtime is present (bundled with installer).
- macOS: Xcode Command Line Tools are required.
