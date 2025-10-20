# Policy Builder UI Guide

## Overview

The Policy Builder delivers an operator-friendly interface for creating, validating, and deploying Edge Policy Hub policies. It combines Monaco-based editing with native integrations to the policy compiler, audit-store, and enforcer services.

## Features

- **Monaco Editor** – Custom DSL highlighting, inline diagnostics, and keyboard shortcuts tailored for policy authors.
- **Real-Time Compilation** – Immediate feedback from the `edge-policy-dsl` compiler, including line/column hints and attribute validation.
- **Test Simulator** – Rich ABAC input form that executes against the enforcer’s decision endpoint for realistic evaluation.
- **Version History** – Full visibility into stored bundles with activation, rollback, and diff capabilities.
- **Deployment Flow** – Safe draft or activate deployments that persist bundles, write Rego artifacts, and trigger hot-reload.

## User Interface

### Editor Panel

- Left pane hosts the Monaco DSL editor.
- Toolbar includes Compile, Deploy, Test, and Version History controls.
- Compilation status bar surfaces success, error counts, last-compiled timestamp, and quick navigation to diagnostics.

### Compiled Rego Panel

- Read-only Monaco viewer displaying the generated Rego.
- Toggle between current compilation, selected historical bundle, or a diff view when both are available.

### Test Simulator Panel

- Dynamic form for subject, action, resource, and environment attributes.
- Preset scenarios (EU residency, quota guardrail) preload realistic inputs.
- Displays allow/deny decision, reason, redaction targets, and evaluation latency.

### Version History Panel

- Table of stored bundles sorted by version.
- Badges highlight active/draft/archived states.
- Actions: **View** loads bundle details, **Activate** promotes, **Rollback** restores previous versions.

## Workflow

### Creating a Policy

1. Navigate to the tenant and open **Policies**.
2. Optionally start from a template (e.g., EU data residency).
3. Edit the DSL and compile; resolve diagnostics inline.
4. Review generated Rego and confirm correctness.
5. Use the Test Simulator with both allow and deny scenarios.
6. Deploy as draft for staging or activate to enforce immediately.

### Testing

1. Fill the ABAC form or load an example input.
2. Submit to hit `/v1/data/tenants/{tenant_id}/allow`.
3. Inspect the decision payload (allow, reason, redact fields).
4. Iterate on the policy and re-test until results match expectations.

### Managing Versions

1. Open Version History to view bundle metadata (author, timestamps, status).
2. Use **View** to inspect Rego and compare with the current compile output.
3. Activate a draft bundle once validated, or rollback to a previous version if issues arise.
4. Deployment events update the enforcer bundle directory and trigger reloads automatically.

## Deployment Options

- **Deploy as Draft** – Persists the bundle but leaves enforcement unchanged, ideal for peer review.
- **Deploy & Activate** – Saves and immediately activates the bundle; requires acknowledgement.
- Both flows write `policy_v{version}.rego` under `config/tenants.d/{tenant}/` and call the enforcer reload endpoint.

## DSL Syntax Highlighting

- Keywords: `allow`, `deny`, `if`, `and`, `or`, `not`, `in`.
- Attribute prefixes: `subject`, `resource`, `environment`, `action`.
- Operators: equality, inequality, comparison operators, and set membership.
- Strings support escape sequences; comments use `#`.
- Matching rules visit `libs/policy-dsl/examples` for practical scripts.

## Error Display

- Parse errors provide line/column markers with red squiggles in Monaco.
- Validation errors call out problematic attributes or unsupported operations.
- Inline summary lists errors and allows jumping to their locations.

## Service Integrations

- **Compilation** – Direct Rust FFI call into the `edge-policy-dsl` crate via Tauri commands.
- **Testing** – Uses the enforcer REST API to evaluate decisions with real data paths.
- **Versioning** – Backed by audit-store’s `PolicyBundleStore`, exposing REST endpoints for CRUD and state transitions.
- **Deployment** – Writes Rego artifacts and signals the enforcer hot-reload API (`/v1/tenants/{tenant}/reload`).

## Troubleshooting

- **Compilation Failure** – Validate syntax matches examples; ensure tenant attributes are spelled correctly.
- **Simulator Errors** – Confirm the enforcer service is reachable and the tenant has an active policy.
- **Deployment Issues** – Verify audit-store and enforcer services are online; check filesystem permissions for the bundle directory.
- **Empty History** – No policies have been deployed for the tenant yet or the audit-store database is new.

## Best Practices

- Compile often to catch syntax and validation errors early.
- Test both positive and negative scenarios to confirm enforcement logic.
- Use descriptive metadata (version, author, description) for traceability.
- Deploy as draft before activating in production environments.
- Keep policies focused; compose multiple policies for complex requirements.
