# Contributing to Edge Policy Hub

Thank you for your interest in contributing to the Edge Policy Hub project. The [README](README.md) describes the project vision and high-level architecture—this guide focuses on how to collaborate effectively within the mono-repo.

## Getting Started
- Fork the repository and clone your fork locally: `git clone git@github.com:<you>/edge-policy-hub.git`
- Install the Rust toolchain defined in `rust-toolchain.toml` (the file pins the stable channel and required components).
- Run `cargo build --workspace` to verify your environment, then explore the crate layout under `services/`, `libs/`, and `apps/`.

## Development Workflow
- Branch naming follows `<type>/<short-description>` where `type` is `feature`, `bugfix`, or `hotfix` (for example, `feature/policy-dsl-parser`).
- We use [Conventional Commits](https://www.conventionalcommits.org/) to keep history clear; e.g., `feat(policy-dsl): add parser for conditions`.
- Submit changes through pull requests. Draft PRs are welcome for early feedback—convert to ready-for-review when the checklist below passes.

## Code Standards
- Always format code with `cargo fmt --all` before pushing.
- Keep the codebase warning-free by running `cargo clippy --workspace --all-features -- -D warnings`.
- Prefer small, focused modules with clear error handling. Add concise comments only when intent is not obvious from the code.

## Testing

Quality gates apply to every pull request:

- Add or update **unit tests** for new modules/functions.
- Provide **integration tests** when API surface or service behaviour changes.
- Extend **E2E tests** if workflows, protocols, or cross-service flows are affected.
- Include **benchmarks** if you touch performance-critical paths (policy evaluation, proxy hot path).
- Keep tests deterministic—avoid time-based sleeps and external network calls.

### Required Commands Before Review

```bash
# Fast path (unit + integration)
make test

# Full validation
make test-all
make bench
make coverage

# OPA/Rego unit tests
make test-opa
```

### Expectations by Change Type

- **New Features**
  - Unit tests covering happy path and edge cases
  - Integration/E2E tests demonstrating the workflow
  - Benchmarks if latency requirements are impacted
- **Bug Fixes**
  - Regression test reproducing the bug before the fix
  - Documentation updates when behaviour changes
- **Documentation**
  - Ensure links remain valid
  - Update examples when introducing new concepts

### Test Quality Standards

- Descriptive names: `test_http_proxy_enforces_eu_residency`
- Clean up resources (temporary directories, spawned processes)
- Assert with readable failure messages
- Prefer event-driven waits (health checks, channels) over sleeps
- Guard slow or environment-heavy tests with `#[ignore]` and document how to run them

For a deeper walkthrough of the test harnesses and benchmarks, see [docs/testing-guide.md](docs/testing-guide.md).

## Building Installers

Before submitting deployment-related changes:

```bash
# Build backend services
cargo build --release --workspace

# Build Tauri installers
cd apps/tauri-ui
pnpm install
pnpm tauri build

# Build Docker images
docker-compose -f infra/docker/docker-compose.yml build
```

Smoke-test the deliverables:

- Linux: `sudo dpkg -i apps/tauri-ui/src-tauri/target/release/bundle/deb/*.deb`
- Windows: run the generated `.exe`/`.msi` installer.
- macOS: install from `.dmg`.
- Docker: `docker-compose -f infra/docker/docker-compose.yml up -d`.
- Systemd: `sudo make install-systemd` / `sudo make uninstall-systemd`.

Deployment PR checklist:

- [ ] Installers build successfully on all platforms you touched.
- [ ] Services start and pass health checks (`/health`).
- [ ] Docker Compose deployment starts and stops cleanly.
- [ ] Uninstall scripts honour data-retention choices.
- [ ] Documentation is updated (installation, deployment, troubleshooting).

## Documentation
- Update crate-level documentation and public API docs when you introduce new features.
- When workflows or architectural concepts change, refresh the relevant sections in `README.md` or add new guides under `docs/`.
- Keep changelog summaries in PR descriptions to ease release note generation.

## Submitting PRs
Use this checklist before requesting review:
- [ ] `cargo build --workspace --all-features`
- [ ] `cargo test --workspace --all-features`
- [ ] `cargo fmt --all`
- [ ] `cargo clippy --workspace --all-features -- -D warnings`
- [ ] Updated documentation, examples, and tests as needed

Once everything passes CI, request review from a maintainer. We appreciate your contributions!
