# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in Resonance, please report it responsibly.

**Do NOT open a public GitHub issue for security vulnerabilities.**

Instead, please send an email to the maintainers with:

1. Description of the vulnerability
2. Steps to reproduce
3. Potential impact
4. Suggested fix (if any)

We will acknowledge receipt within 48 hours and provide a detailed response within 7 days.

## Security Best Practices

When deploying Resonance:

### Required Configuration

- **JWT_SECRET**: Use a strong, random secret (min 32 characters)
- **MEILISEARCH_KEY**: Generate a unique master key
- **DB_PASSWORD**: Use a strong database password
- **CORS_ORIGINS**: Configure explicit allowed origins for production

### Recommended Practices

- Run behind a reverse proxy (Traefik, Nginx, Caddy)
- Enable HTTPS with valid certificates
- Use network isolation for database and Redis
- Regularly update dependencies
- Monitor logs for suspicious activity
- Enable rate limiting on authentication endpoints

### Environment Variables

Never commit `.env` files or secrets to version control. Use:
- Environment variables in production
- Secrets management (Docker secrets, Vault, etc.)
- `.env.example` as a template only

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.x.x   | :white_check_mark: |

## Security Features

Resonance includes several security features:

- **JWT Authentication**: HTTP-only cookies, refresh tokens
- **Row-Level Security**: PostgreSQL RLS for multi-tenant isolation
- **Input Validation**: Serde (Rust) and Zod (TypeScript)
- **Rate Limiting**: Configurable per-endpoint limits
- **CORS**: Environment-configured allowed origins
- **Password Hashing**: Argon2id for password storage
