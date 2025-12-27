## Description

<!-- Provide a clear and concise description of your changes -->

### What does this PR do?


### Why is this change needed?


### Related Issues

<!-- Link any related issues using "Fixes #123" or "Relates to #123" -->


---

## Type of Change

<!-- Mark the appropriate option with an "x" -->

- [ ] Bug fix (non-breaking change that fixes an issue)
- [ ] New feature (non-breaking change that adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to change)
- [ ] Performance improvement
- [ ] Refactoring (no functional changes)
- [ ] Documentation update
- [ ] CI/CD or infrastructure change
- [ ] Dependency update

---

## Checklist

### Code Quality

- [ ] My code follows the project's code style guidelines
- [ ] I have run `cargo fmt` and `cargo clippy` (for Rust changes)
- [ ] I have run `pnpm lint` and `pnpm typecheck` (for TypeScript changes)
- [ ] I have added comments for complex or non-obvious code
- [ ] I have removed any debugging code, console logs, or commented-out code

### Testing

- [ ] I have added tests that prove my fix/feature works
- [ ] New and existing unit tests pass locally (`cargo test` / `pnpm test`)
- [ ] I have tested my changes manually
- [ ] I have tested edge cases and error scenarios

<!-- For UI changes -->
- [ ] I have tested on different screen sizes (if applicable)
- [ ] I have tested keyboard navigation and accessibility (if applicable)

### Documentation

- [ ] I have updated the README if needed
- [ ] I have updated CLAUDE.md if this affects development workflow
- [ ] I have added/updated inline documentation and type definitions
- [ ] I have updated API documentation (GraphQL schema, OpenAPI, etc.) if applicable

### Database Changes (if applicable)

- [ ] I have created a migration file using `cargo sqlx migrate add <name>`
- [ ] Migration is reversible or I have documented why it isn't
- [ ] I have run `cargo sqlx prepare --workspace` to update query cache
- [ ] I have tested the migration on a fresh database

---

## Breaking Changes

<!-- If this PR introduces breaking changes, describe them here -->

### Does this PR introduce breaking changes?

- [ ] Yes
- [ ] No

<!-- If yes, complete the following -->

### Breaking Change Details

**What breaks:**


**Migration path for users:**


**Deprecation notices added:** Yes / No / N/A

---

## Screenshots / Recordings

<!-- Add screenshots or recordings for UI changes -->

| Before | After |
|--------|-------|
|        |       |

---

## Performance Impact

<!-- Describe any performance implications of this change -->

- [ ] This change has no significant performance impact
- [ ] This change improves performance (describe below)
- [ ] This change may impact performance (describe below)

**Details:**


---

## Deployment Notes

<!-- Any special deployment considerations? -->

- [ ] No special deployment steps required
- [ ] Requires environment variable changes (list below)
- [ ] Requires database migration
- [ ] Requires cache invalidation
- [ ] Requires service restart

**Deployment steps:**


---

## Additional Notes

<!-- Any additional context or notes for reviewers -->

