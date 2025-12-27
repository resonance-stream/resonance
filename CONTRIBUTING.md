# Contributing to Resonance

Thank you for your interest in contributing to Resonance! This document provides guidelines and instructions for contributing.

## Code of Conduct

By participating in this project, you agree to maintain a respectful and inclusive environment for everyone.

## Getting Started

### Prerequisites

- **Rust** 1.75+ with `cargo`
- **Node.js** 18+ with `pnpm`
- **Docker** and **Docker Compose**
- **PostgreSQL** client (for running migrations)

### Development Setup

1. **Clone the repository**
   ```bash
   git clone https://github.com/resonance-stream/resonance.git
   cd resonance
   ```

2. **Copy environment configuration**
   ```bash
   cp .env.example .env
   # Edit .env with your local settings
   ```

3. **Start infrastructure services**
   ```bash
   docker compose up -d postgres redis meilisearch
   ```

4. **Run database migrations**
   ```bash
   cd apps/api
   cargo sqlx migrate run
   ```

5. **Install frontend dependencies**
   ```bash
   pnpm install
   ```

6. **Start development servers**
   ```bash
   # Terminal 1: Backend API
   cargo run -p resonance-api

   # Terminal 2: Frontend
   cd apps/web && pnpm dev
   ```

## How to Contribute

### Reporting Bugs

1. Check existing issues to avoid duplicates
2. Use the bug report template
3. Include:
   - Clear description of the issue
   - Steps to reproduce
   - Expected vs actual behavior
   - Environment details (OS, browser, versions)

### Suggesting Features

1. Check existing issues and discussions
2. Use the feature request template
3. Explain the use case and proposed solution

### Submitting Code

1. **Fork and clone** the repository
2. **Create a branch** from `main`:
   ```bash
   git checkout -b feat/your-feature-name
   # or
   git checkout -b fix/your-bug-fix
   ```

3. **Make your changes** following our coding standards

4. **Write tests** for new functionality

5. **Run checks locally**:
   ```bash
   # Rust
   cargo fmt
   cargo clippy -- -D warnings
   cargo test

   # Frontend
   pnpm lint
   pnpm typecheck
   pnpm test
   ```

6. **Commit with conventional commits**:
   ```bash
   git commit -m "feat(playback): add crossfade support"
   ```

7. **Push and create a Pull Request**

## Coding Standards

### Rust

- Follow `rustfmt` defaults
- Use `thiserror` for error types in libraries
- Use `anyhow` for error handling in binaries
- Use `tracing` for all logging
- Keep handlers thin; business logic belongs in services
- Write doc comments for public APIs

### TypeScript

- Strict mode enabled
- Prefer functional components with hooks
- Use `const` by default
- Explicit return types on exported functions
- Avoid `any`; use `unknown` for truly unknown types

### Commit Messages

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code changes that neither fix bugs nor add features
- `perf`: Performance improvements
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

**Scopes:**
- `api`: Backend API
- `worker`: Background worker
- `web`: Frontend
- `db`: Database/migrations
- `docker`: Docker configuration
- `deps`: Dependencies

## Pull Request Guidelines

- Keep PRs focused on a single change
- Update documentation if needed
- Add tests for new functionality
- Ensure all checks pass
- Request review from maintainers

## Project Structure

```
resonance/
├── .github/
│   ├── actions/          # Reusable composite actions
│   │   ├── rust-setup/   # Rust toolchain setup
│   │   └── setup-node-pnpm/  # Node.js + pnpm setup
│   └── workflows/        # CI/CD workflows
├── apps/
│   ├── api/          # Rust backend (Axum)
│   ├── worker/       # Background job processor
│   └── web/          # React frontend
├── packages/
│   ├── shared-config/    # Shared Rust configuration
│   └── shared-types/     # Shared TypeScript types
└── docker/           # Docker configurations
```

## CI/CD Workflows

The project uses GitHub Actions for continuous integration and deployment. Key workflows:

| Workflow | Purpose |
|----------|---------|
| `ci.yml` | Build, test, and lint on every PR |
| `security.yml` | Vulnerability scanning (daily + on PR) |
| `release.yml` | Automated releases via release-please |
| `docker.yml` | Build and publish Docker images |

### Reusable Components

When adding new workflows, use the existing composite actions for consistency:

```yaml
# For Rust jobs
- uses: ./.github/actions/rust-setup
  with:
    components: clippy  # Optional

# For Node.js/pnpm jobs
- uses: ./.github/actions/setup-node-pnpm
```

See [.github/workflows/README.md](.github/workflows/README.md) for complete documentation.

## Getting Help

- Check the [README](README.md) for setup instructions
- Review [CLAUDE.md](CLAUDE.md) for architecture details
- Open a discussion for questions

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
