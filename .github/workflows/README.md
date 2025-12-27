# GitHub Workflows

This document describes all GitHub Actions workflows configured for the Resonance project.

## CI Badges

[![CI](https://github.com/resonance-stream/resonance/actions/workflows/ci.yml/badge.svg)](https://github.com/resonance-stream/resonance/actions/workflows/ci.yml)
[![Security Scanning](https://github.com/resonance-stream/resonance/actions/workflows/security.yml/badge.svg)](https://github.com/resonance-stream/resonance/actions/workflows/security.yml)
[![Docker Build](https://github.com/resonance-stream/resonance/actions/workflows/docker.yml/badge.svg)](https://github.com/resonance-stream/resonance/actions/workflows/docker.yml)
[![Release](https://github.com/resonance-stream/resonance/actions/workflows/release.yml/badge.svg)](https://github.com/resonance-stream/resonance/actions/workflows/release.yml)

---

## Workflow Overview

| Workflow | File | Purpose | Triggers |
|----------|------|---------|----------|
| [CI](#ci) | `ci.yml` | Build, test, and lint | Push/PR to `main` |
| [Security Scanning](#security-scanning) | `security.yml` | Vulnerability scanning | Push/PR to `main`, daily schedule, manual |
| [Release](#release) | `release.yml` | Automated releases via release-please | Push to `main`, manual |
| [Docker Build](#docker-build) | `docker.yml` | Build and publish Docker images | Version tags (`v*`), manual |
| [Auto Label Issues](#auto-label-issues) | `auto-label-issues.yml` | Label issues based on keywords | Issue opened/edited |
| [PR Labeler](#pr-labeler) | `labeler.yml` | Label PRs based on changed files | PR opened/synchronized/reopened |

---

## Reusable Components

The Resonance project uses reusable GitHub Actions components to reduce duplication and ensure consistency across workflows.

### Composite Actions

Composite actions are located in `.github/actions/` and encapsulate common setup steps:

| Action | Location | Purpose |
|--------|----------|---------|
| **rust-setup** | `.github/actions/rust-setup/` | Sets up Rust toolchain with caching, optional components (clippy, rustfmt), and configurable cache keys |
| **setup-node-pnpm** | `.github/actions/setup-node-pnpm/` | Sets up Node.js and pnpm with caching, optionally installs dependencies |

**Usage Example (Rust):**
```yaml
- name: Setup Rust
  uses: ./.github/actions/rust-setup
  with:
    components: clippy,rustfmt  # Optional: rustfmt, clippy
    cache-key: build            # Optional: cache key suffix
```

**Usage Example (Node.js/pnpm):**
```yaml
- name: Setup Node.js and pnpm
  uses: ./.github/actions/setup-node-pnpm
  with:
    node-version: '20'          # Optional: defaults to 20
    pnpm-version: '9'           # Optional: defaults to 9
    install-dependencies: true  # Optional: defaults to true
```

### Reusable Workflows

Reusable workflows use the `workflow_call` trigger and are invoked from other workflows:

| Workflow | File | Purpose |
|----------|------|---------|
| **Docker Build** | `docker-build.yml` | Builds and pushes Docker images to GHCR with multi-platform support |

**Usage Example:**
```yaml
jobs:
  build-api:
    uses: ./.github/workflows/docker-build.yml
    with:
      image-name: resonance-api
      dockerfile: ./docker/Dockerfile
      context: .
    secrets: inherit
```

### When to Use Each Pattern

| Pattern | Use When |
|---------|----------|
| **Composite Action** | Reusing setup steps (toolchain, caching, dependencies) across multiple jobs |
| **Reusable Workflow** | Reusing entire job definitions with consistent inputs/outputs |

---

## CI

**File:** [`ci.yml`](ci.yml)

Runs continuous integration checks for both Rust backend and TypeScript frontend.

### Trigger Conditions

| Trigger | Condition |
|---------|-----------|
| `push` | Branches: `main` |
| `pull_request` | Branches: `main` |

### Jobs

| Job | Description | Dependencies |
|-----|-------------|--------------|
| `rust-fmt` | Check Rust code formatting | None |
| `rust-clippy` | Run Clippy linter | None |
| `rust-test` | Run Rust unit tests (with PostgreSQL and Redis services) | None |
| `typescript-lint` | Run ESLint on frontend code | None |
| `typescript-typecheck` | TypeScript type checking | None |
| `typescript-test` | Run Vitest tests | None |
| `build` | Full build verification | `rust-fmt`, `rust-clippy`, `typescript-lint`, `typescript-typecheck` |

### Features

- Concurrency control: Cancels in-progress runs for the same branch
- Caching: Cargo registry and pnpm store caching for faster builds
- Service containers: PostgreSQL (pgvector) and Redis for integration tests

---

## Security Scanning

**File:** [`security.yml`](security.yml)

Comprehensive security vulnerability scanning for dependencies and code.

### Trigger Conditions

| Trigger | Condition |
|---------|-----------|
| `push` | Branches: `main` |
| `pull_request` | Branches: `main` |
| `schedule` | Daily at 2:00 AM UTC (`0 2 * * *`) |
| `workflow_dispatch` | Manual trigger |

### Jobs

| Job | Description | Tools |
|-----|-------------|-------|
| `cargo-audit` | Rust dependency vulnerability scanning | cargo-audit |
| `cargo-deny` | Rust license and advisory checking | cargo-deny |
| `npm-audit` | npm/pnpm dependency scanning | pnpm audit |
| `codeql` | Static code analysis (JavaScript/TypeScript) | GitHub CodeQL |
| `codeql-rust` | Static code analysis (Rust, beta) | GitHub CodeQL |
| `trivy` | Filesystem and secrets scanning | Trivy |
| `security-summary` | Aggregates results summary | N/A |

### Artifacts

- `cargo-audit-report` - JSON report of Rust vulnerabilities (30-day retention)
- `npm-audit-report` - JSON report of npm vulnerabilities (30-day retention)
- SARIF results uploaded to GitHub Security tab

### Permissions Required

- `contents: read`
- `security-events: write`
- `actions: read`

---

## Release

**File:** [`release.yml`](release.yml)

Automated release management using [release-please](https://github.com/googleapis/release-please).

### Trigger Conditions

| Trigger | Condition |
|---------|-----------|
| `push` | Branches: `main` |
| `workflow_dispatch` | Manual trigger |

### Jobs

| Job | Description | Runs When |
|-----|-------------|-----------|
| `release-please` | Creates/updates release PRs, generates changelogs | Always |
| `publish-docker` | Builds and publishes Docker images | On release created |
| `build-binaries` | Builds release binaries for multiple platforms | On release created |
| `update-release-notes` | Adds installation instructions to release notes | After Docker and binaries complete |

### Release Artifacts

**Docker Images (pushed to ghcr.io):**
- `resonance-api` - API server image
- `resonance-worker` - Background worker image

**Binaries (attached to GitHub Release):**
| Platform | Architecture | File |
|----------|--------------|------|
| Linux | x86_64 | `resonance-linux-amd64.tar.gz` |
| Linux | ARM64 | `resonance-linux-arm64.tar.gz` |
| macOS | x86_64 | `resonance-darwin-amd64.tar.gz` |
| macOS | ARM64 (Apple Silicon) | `resonance-darwin-arm64.tar.gz` |

### Configuration Files

- `release-please-config.json` - Release-please configuration
- `.release-please-manifest.json` - Version manifest

### Permissions Required

- `contents: write`
- `pull-requests: write`
- `packages: write`

---

## Docker Build

**File:** [`docker.yml`](docker.yml)

Builds and publishes Docker images to GitHub Container Registry.

### Trigger Conditions

| Trigger | Condition |
|---------|-----------|
| `push` | Tags: `v*` (version tags) |
| `workflow_dispatch` | Manual trigger (optional custom tag input) |

### Jobs

| Job | Description |
|-----|-------------|
| `build-api` | Build and push API server image |
| `build-worker` | Build and push background worker image |
| `release-summary` | Generate summary for tag releases |

### Image Tags

For release tags (e.g., `v1.2.3`):
- `1.2.3` - Full version
- `1.2` - Major.minor
- `1` - Major only
- `latest` - Latest stable release

For manual runs:
- `sha-<commit>` - Git commit SHA
- Custom tag (if provided via input)

### Platforms

- `linux/amd64`
- `linux/arm64`

### Permissions Required

- `contents: read`
- `packages: write`

---

## Auto Label Issues

**File:** [`auto-label-issues.yml`](auto-label-issues.yml)

Automatically labels issues based on keywords in title and body.

### Trigger Conditions

| Trigger | Condition |
|---------|-----------|
| `issues` | Types: `opened`, `edited` |

### Labels Applied

| Label | Keywords |
|-------|----------|
| `bug` | bug, error, crash, broken, fix, issue, problem, fail, exception, not working |
| `enhancement` | feature, request, enhancement, add, new, implement, support, would be nice, suggestion |
| `documentation` | documentation, docs, readme, wiki, guide, tutorial, example, typo |
| `question` | question, help, how to, how do, what is, why does, confused, unclear |
| `performance` | performance, slow, memory, cpu, optimize, speed, latency, lag |
| `security` | security, vulnerability, cve, exploit, authentication, authorization, xss, injection, csrf |
| `ui/ux` | ui, ux, design, layout, style, css, theme, dark mode, light mode, responsive, mobile |
| `api` | api, graphql, rest, endpoint, route, request, response, websocket |
| `database` | database, postgresql, postgres, redis, migration, query, sql, meilisearch |
| `playback` | playback, audio, streaming, gapless, crossfade, equalizer, eq, volume, buffer |
| `ai` | ai, recommendation, mood, ollama, embedding, vector, natural language, nlp |
| `sync` | sync, synchronization, real-time, cross-device, multi-device |
| `pwa` | pwa, offline, service worker, cache, install, manifest |
| `integration` | lidarr, integration, import, library, scan |
| `deployment` | docker, container, compose, deployment, kubernetes, k8s, helm |
| `frontend` | frontend, react, typescript, vite, component, hook, zustand, radix |
| `backend` | backend, rust, axum, server, tokio, sqlx |

### Permissions Required

- `issues: write`

---

## PR Labeler

**File:** [`labeler.yml`](labeler.yml)

Automatically labels pull requests based on changed file paths.

### Trigger Conditions

| Trigger | Condition |
|---------|-----------|
| `pull_request_target` | Types: `opened`, `synchronize`, `reopened` |

### Configuration

Labels are configured in [`.github/labeler.yml`](../labeler.yml). The labeler uses file path patterns to determine which labels to apply.

### Features

- `sync-labels: true` - Removes labels that no longer match changed files

### Permissions Required

- `contents: read`
- `pull-requests: write`

---

## Required Secrets and Variables

### Secrets

| Secret | Used By | Description |
|--------|---------|-------------|
| `GITHUB_TOKEN` | All workflows | Automatic GitHub token (provided by Actions) |
| `RELEASE_APP_PRIVATE_KEY` | Release | GitHub App private key for release-please (optional) |

### Variables

| Variable | Used By | Description |
|----------|---------|-------------|
| `RELEASE_APP_ID` | Release | GitHub App ID for release-please (optional) |

---

## Workflow Dependencies

```
                    ┌─────────────────────────┐
                    │     Push to main        │
                    └───────────┬─────────────┘
                                │
         ┌──────────────────────┼──────────────────────┐
         │                      │                      │
         ▼                      ▼                      ▼
   ┌───────────┐        ┌──────────────┐       ┌──────────────┐
   │    CI     │        │   Security   │       │   Release    │
   │  (tests)  │        │  (scanning)  │       │  (release-   │
   │           │        │              │       │   please)    │
   └───────────┘        └──────────────┘       └──────┬───────┘
                                                      │
                                               (on release)
                                                      │
                               ┌──────────────────────┼───────────────────────┐
                               │                      │                       │
                               ▼                      ▼                       ▼
                        ┌──────────────┐      ┌──────────────┐       ┌────────────────┐
                        │   Publish    │      │    Build     │       │    Update      │
                        │   Docker     │      │   Binaries   │       │  Release Notes │
                        └──────────────┘      └──────────────┘       └────────────────┘
```

---

## Troubleshooting

### CI Failures

1. **Rust formatting errors:** Run `cargo fmt --all` locally before pushing
2. **Clippy warnings:** Run `cargo clippy --all-targets --all-features -- -D warnings` locally
3. **TypeScript lint errors:** Run `pnpm lint` in the `apps/web` directory
4. **Type check failures:** Run `pnpm typecheck` in the `apps/web` directory

### Security Scan Failures

1. **Cargo audit failures:** Update vulnerable dependencies or add exceptions in `deny.toml`
2. **npm audit failures:** Run `pnpm audit fix` to auto-fix vulnerabilities
3. **CodeQL alerts:** Review findings in the Security tab and fix code issues

### Release Issues

1. **Release-please not creating PR:** Ensure commits follow [Conventional Commits](https://www.conventionalcommits.org/) format
2. **Docker build failures:** Check Dockerfile syntax and build context
3. **Binary build failures:** Ensure cross-compilation tools are properly configured

### Labeling Issues

1. **Labels not applied:** Ensure the labels exist in repository settings
2. **Wrong labels:** Review keyword mappings in `issue-label-config.yml`
3. **PR labels missing:** Check `.github/labeler.yml` file path patterns
