# Tauri UI Architecture Guide

## Architecture Overview
The Edge Policy Hub Tauri UI combines a web frontend with a Rust backend to deliver a secure desktop experience. The app uses Tauri 2.0, which embeds a lightweight Rust runtime alongside a WebView. All sensitive operations—service discovery, HTTP requests, and secrets—occur inside the Rust side, while the React frontend focuses on user interactions. Communication between the layers happens through Tauri's `invoke` API, allowing type-safe command calls without exposing internal service endpoints to the browser context.

## Frontend Architecture
- **Routing**: React Router organizes the app into list, create, and edit routes under `src/pages`.
- **Forms**: React Hook Form coordinates with Zod schemas to validate tenant payloads. Validation errors surface instantly without round-trips.
- **State Management**: TanStack Query caches server state (tenant collections, individual tenants) and handles refetching after mutations.
- **Components**: `TenantConfigForm` encapsulates quota and feature controls, ensuring both create and edit forms stay in sync.
- **Styling**: Minimal CSS in `App.css` establishes a clean workspace-style layout with sidebar navigation and content cards.

## Backend Architecture
- **Command Modules**: `src-tauri/src/commands.rs` houses async commands exported to the frontend. Each command wraps outbound HTTP calls, marshals payloads, and normalizes errors into `CommandError`.
- **Configuration**: `ServiceConfig` mirrors the repository’s pattern from other services, inheriting defaults and environment overrides with validation.
- **HTTP Client**: A shared `reqwest::Client` per command applies timeouts defined in configuration. `tauri-plugin-http` grants the necessary permissions at runtime.
- **Error Handling**: `CommandError` categorizes validation, network, serialization, and API failures. The enum implements `Serialize`, ensuring friendly error strings reach the UI.
- **Security**: Tauri capabilities limit window permissions and restrict HTTP requests to known internal services.

## Tenant Configuration Schema
The tenant `config` field inside audit-store is persisted as JSON. The UI expects the following structure:

```json
{
  "quotas": {
    "message_limit": 50000,
    "bandwidth_limit_gb": 100.0
  },
  "features": {
    "data_residency": ["EU", "US"],
    "pii_redaction": true
  }
}
```

- `quotas.message_limit`: maximum MQTT messages per day (integer).
- `quotas.bandwidth_limit_gb`: monthly bandwidth allocation in gigabytes (floating-point).
- `features.data_residency`: allowed regions for data residency (EU, US, APAC).
- `features.pii_redaction`: toggle for automated removal of sensitive information in logs.

## API Integration
- **Audit Store** (`http://127.0.0.1:8182`):
  - `GET /api/tenants`: list tenants, optionally filtered by `status`.
  - `GET /api/tenants/{tenant_id}`: retrieve a single tenant.
  - `POST /api/tenants`: create a tenant; body contains `tenant_id`, `name`, and `config`.
  - `PUT /api/tenants/{tenant_id}`: update name, status, or configuration.
  - `DELETE /api/tenants/{tenant_id}`: soft delete tenant records.
- **Quota Tracker** (`http://127.0.0.1:8183`):
  - `POST /api/quota/limits`: register or update quota limits using `SetLimitsRequest`.

Every command composes URLs with validation, executes requests via `reqwest`, and maps non-success HTTP codes to actionable error messages. TanStack Query relies on these commands to refresh the UI and maintain cache consistency.

## Form Validation
- `TenantCreatePage` schema enforces:
  - `tenant_id`: 1–64 characters, alphanumeric with underscores or hyphens.
  - `name`: 1–255 characters.
  - `message_limit`: numeric ≥ 1.
  - `bandwidth_limit_gb`: numeric ≥ 0.1.
  - `data_residency`: subset of predefined regions.
  - `pii_redaction`: boolean toggle.
- `TenantEditPage` reuses quota and feature rules, and adds a required `status` enum.

All forms provide inline error messages through React Hook Form’s error bag, preventing invalid submissions from reaching the backend.

## State Management
- Queries:
  - `["tenants", status]` fetches tenant collections with optional status filters.
  - `["tenant", tenantId]` hydrates edit forms.
- Mutations (`createTenant`, `updateTenant`, `deleteTenant`) invalidate relevant queries to guarantee fresh data on subsequent views.
- Default stale time is five minutes, keeping frequently accessed tenant data responsive while minimizing redundant network calls.

## Development Workflow
1. Start backend services (`audit-store`, `quota-tracker`).
2. Run `npm run tauri dev` from `apps/tauri-ui/` to launch the desktop shell.
3. Implement frontend or backend changes under `src/` or `src-tauri/`.
4. Use TanStack Query devtools (optional add-on) for cache inspection when debugging.
5. Run `npm run tauri build` before releasing to ensure the native bundle compiles across targets.

## Testing Strategy
- **Rust**: Add unit tests for command helpers (e.g., config parsing, error mapping) and integration tests with mocked HTTP servers using `httptest` or `wiremock`.
- **Frontend**: Leverage React Testing Library to validate form validation, mutation flows, and rendering of error states.
- **End-to-End**: Consider Playwright or Tauri’s testing harness to automate smoke tests across supported operating systems.

## Security Considerations
- Tauri capabilities whitelist only the main window and explicit HTTP endpoints, limiting exposure.
- Input validation occurs on both frontend (Zod) and backend (Rust command + audit-store service).
- Future hardening can add token-based auth for service calls or TLS termination behind a local proxy.

## Troubleshooting
- **Service unreachable**: Verify audit-store/quota-tracker are running and accessible on localhost; update `.env` if ports differ.
- **Timeouts**: Increase `REQUEST_TIMEOUT_SECS` for remote deployments with higher latency.
- **Permission errors**: Regenerate icons or capabilities after upgrading Tauri; stale files can block bundling.
- **Validation failures**: Review inline error messages. Tenant IDs must remain unique and respect the allowed character set.

Refer back to the root `README.md` and individual service documentation for broader ecosystem context.
