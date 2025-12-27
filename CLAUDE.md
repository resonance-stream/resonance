# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Resonance** is a self-hosted Spotify-like music streaming platform featuring:
- Gapless playback with crossfade and 10-band equalizer
- AI-powered recommendations, mood detection, and natural language search
- Real-time cross-device synchronization
- Lidarr integration for automatic music library management
- PWA with unlimited offline caching

**Tech Stack:**
- Backend: Rust + Axum
- Frontend: React + TypeScript + Vite
- Database: PostgreSQL 16 + pgvector
- Cache: Redis 7
- Search: Meilisearch
- AI: Ollama (user-specified model)
- Deployment: Docker Compose

---

## Repository Structure

```
resonance/
├── apps/
│   ├── api/                    # Rust backend (Axum)
│   │   ├── src/
│   │   │   ├── routes/         # HTTP/GraphQL endpoints
│   │   │   ├── services/       # Business logic
│   │   │   ├── models/         # Database models (SQLx)
│   │   │   ├── websocket/      # Real-time sync handlers
│   │   │   └── graphql/        # GraphQL schema definitions
│   │   ├── migrations/         # SQLx migrations
│   │   └── Cargo.toml
│   │
│   ├── worker/                 # Background job processor
│   │   └── src/
│   │       └── jobs/           # Scheduled tasks (recommendations, downloads)
│   │
│   └── web/                    # React frontend
│       └── src/
│           ├── components/     # UI components (Radix UI based)
│           ├── hooks/          # Custom React hooks
│           ├── stores/         # Zustand state stores
│           ├── lib/            # Utilities and helpers
│           └── pages/          # Route pages
│
├── packages/
│   ├── shared-config/          # Shared Rust configuration types
│   └── shared-types/           # Shared TypeScript types (GraphQL codegen)
│
├── docker/
│   ├── Dockerfile              # API production image
│   ├── Dockerfile.worker       # Worker production image
│   └── init.sql                # Database initialization
│
├── docker-compose.yml          # Full stack deployment
├── docker-compose.dev.yml      # Development overrides
├── Cargo.toml                  # Workspace root
├── pnpm-workspace.yaml         # pnpm workspace config
└── .env.example                # Environment template
```

---

## Key Technologies

### Backend (Rust)
| Crate | Purpose |
|-------|---------|
| `axum` | Web framework |
| `tokio` | Async runtime |
| `sqlx` | PostgreSQL driver (compile-time checked queries) |
| `async-graphql` | GraphQL server |
| `symphonia` | Audio decoding |
| `redis` | Caching + pub/sub |
| `meilisearch-sdk` | Search integration |
| `thiserror` | Error handling |
| `tracing` | Structured logging |

### Frontend (React)
| Package | Purpose |
|---------|---------|
| `react` 18 | UI framework |
| `typescript` | Type safety (strict mode) |
| `vite` | Build tool |
| `@tanstack/react-query` | Server state management |
| `zustand` | Client state management |
| `@radix-ui/*` | Accessible UI primitives |
| `workbox` | PWA/offline support |

### Infrastructure
| Service | Image | Purpose |
|---------|-------|---------|
| PostgreSQL | `pgvector/pgvector:pg16` | Primary DB + vector embeddings |
| Redis | `redis:7-alpine` | Cache, sessions, pub/sub |
| Meilisearch | `getmeili/meilisearch:v1.6` | Full-text search |
| Ollama | `ollama/ollama` | Local AI inference |

---

## Development Commands

### Rust Backend
```bash
# Build all Rust packages
cargo build

# Run API server (from apps/api)
cargo run -p resonance-api

# Run worker (from apps/worker)
cargo run -p resonance-worker

# Run all tests
cargo test

# Run a single test by name
cargo test test_name

# Run tests in a specific package
cargo test -p resonance-api

# Run tests with coverage
cargo tarpaulin

# Format code
cargo fmt

# Lint
cargo clippy -- -D warnings

# Check SQLx queries (requires DATABASE_URL)
cargo sqlx prepare --workspace
```

### Frontend
```bash
# Install dependencies
pnpm install

# Start dev server (from apps/web)
pnpm dev

# Build for production
pnpm build

# Run all tests
pnpm test

# Run tests in watch mode
pnpm test:watch

# Run a single test file
pnpm test src/stores/playerStore.test.ts

# Run tests matching a pattern
pnpm test -- -t "pattern"

# Lint
pnpm lint

# Type check
pnpm typecheck

# Generate GraphQL types
pnpm codegen
```

### Docker Compose
```bash
# Start all services
docker compose up -d

# Start with rebuild
docker compose up -d --build

# View logs
docker compose logs -f [service]

# Stop all services
docker compose down

# Reset database
docker compose down -v && docker compose up -d
```

### Database Migrations
```bash
# Create migration
cargo sqlx migrate add <name>

# Run migrations
cargo sqlx migrate run

# Revert last migration
cargo sqlx migrate revert
```

---

## Code Style

### Rust
- Follow `rustfmt` defaults
- Use `thiserror` for error types
- Prefer `anyhow` in binaries, `thiserror` in libraries
- Use `tracing` for all logging
- Organize modules: models → services → routes
- Keep handlers thin, business logic in services

```rust
// Error handling pattern
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PlaybackError {
    #[error("track not found: {0}")]
    TrackNotFound(i64),
    #[error("transcoding failed: {0}")]
    TranscodingFailed(#[from] std::io::Error),
}
```

### TypeScript
- Strict mode enabled
- Prefer functional components with hooks
- Use `const` by default, `let` only when needed
- Explicit return types on functions
- Avoid `any`, use `unknown` for truly unknown types

```typescript
// Component pattern
export function TrackCard({ track }: TrackCardProps): JSX.Element {
  const playTrack = usePlayerStore((s) => s.playTrack);

  return (
    <Card onClick={() => playTrack(track)}>
      {/* ... */}
    </Card>
  );
}
```

### Commit Messages
Use [Conventional Commits](https://www.conventionalcommits.org/):
```
feat(playback): add gapless playback support
fix(auth): resolve token refresh race condition
docs(readme): update installation steps
refactor(api): extract streaming logic to service
test(player): add crossfade unit tests
chore(deps): update dependencies
```

---

## Architecture Patterns

### Backend: Modular Monolith
```
Request → Router → Handler → Service → Repository → Database
                      ↓
                  Response
```

- **Handlers**: Parse requests, call services, format responses
- **Services**: Business logic, orchestration
- **Repositories**: Database access (via SQLx)
- **Models**: Domain types and database entities

### API Design
- **GraphQL** for most queries (library, search, recommendations)
- **REST** for streaming (`GET /stream/:trackId`)
- **WebSocket** for real-time sync

### Real-Time Sync
```
Device A ──┐
           │     ┌─────────────┐
Device B ──┼────►│ Redis PubSub│◄──► WebSocket Server
           │     │ user:{id}:* │
Device C ──┘     └─────────────┘
```

### State Management (Frontend)
- **Server State**: TanStack Query for caching, refetching
- **Client State**: Zustand for player, UI state
- **Sync State**: WebSocket subscription for cross-device

### Background Worker Jobs
The worker (`apps/worker`) handles scheduled background tasks:
- `library_scan.rs` - Scans music library for new/changed files
- `feature_extraction.rs` - Extracts audio features for recommendations
- `embedding_generation.rs` - Generates AI embeddings via Ollama
- `weekly_playlist.rs` - Creates weekly discovery playlists
- `lidarr_sync.rs` - Syncs with Lidarr for library management
- `prefetch.rs` - Smart prefetch for autoplay queue

---

## Testing

### Rust
- Unit tests: Inline in `src/` files (run with `cargo test`)
- Integration tests: `apps/api/tests/` (require running database)
- Use `cargo test --test '*' -- --test-threads=1` for integration tests

### Frontend
- **Vitest** for test runner
- **React Testing Library** for component testing
- **MSW** for API mocking
- Test files: Co-located with source (e.g., `playerStore.test.ts`)

---

## Common Tasks

### Adding a New API Endpoint

1. **Define GraphQL schema** (`apps/api/src/graphql/schema.graphql`):
```graphql
extend type Query {
  playlist(id: ID!): Playlist
}
```

2. **Create resolver** (`apps/api/src/graphql/resolvers/playlist.rs`):
```rust
#[Object]
impl PlaylistQuery {
    async fn playlist(&self, ctx: &Context<'_>, id: ID) -> Result<Playlist> {
        let service = ctx.data::<PlaylistService>()?;
        service.get_by_id(id.parse()?).await
    }
}
```

3. **Implement service** (`apps/api/src/services/playlist.rs`)
4. **Add tests**
5. **Regenerate frontend types**: `pnpm codegen`

### Adding a New React Component

1. **Create component** (`apps/web/src/components/TrackList/TrackList.tsx`):
```typescript
export interface TrackListProps {
  tracks: Track[];
  onTrackClick?: (track: Track) => void;
}

export function TrackList({ tracks, onTrackClick }: TrackListProps): JSX.Element {
  return (
    <div className="track-list">
      {tracks.map((track) => (
        <TrackCard key={track.id} track={track} onClick={onTrackClick} />
      ))}
    </div>
  );
}
```

2. **Add index export** (`apps/web/src/components/TrackList/index.ts`)
3. **Write tests** (`apps/web/src/components/TrackList/TrackList.test.tsx`)

### Database Migrations

```bash
# Create migration
cargo sqlx migrate add add_lyrics_table

# Edit migration file in apps/api/migrations/
# Run migration
cargo sqlx migrate run

# Update SQLx query cache
cargo sqlx prepare --workspace
```

---

## Environment Variables

Copy `.env.example` to `.env` and configure. Key variables:

### Required
| Variable | Description |
|----------|-------------|
| `DB_PASSWORD` | PostgreSQL password (generates `DATABASE_URL`) |
| `JWT_SECRET` | Secret for JWT signing (min 32 chars) |
| `MEILISEARCH_KEY` | Meilisearch master key |
| `LIDARR_URL` | Lidarr instance URL |
| `LIDARR_API_KEY` | Lidarr API key |
| `MUSIC_LIBRARY_PATH` | Path to music files |

### Optional
| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | `8080` | API server port |
| `REDIS_URL` | `redis://redis:6379` | Redis connection |
| `OLLAMA_URL` | `http://ollama:11434` | Ollama API URL |
| `OLLAMA_MODEL` | `mistral` | Ollama model to use |
| `RUST_LOG` | `info` | Logging verbosity (e.g., `debug`, `resonance_api=debug,sqlx=warn`) |

---

## Debugging Tips

### Rust Backend
```bash
# Enable debug logging
RUST_LOG=debug cargo run

# SQL query logging
RUST_LOG=sqlx=trace cargo run

# Backtrace on panic
RUST_BACKTRACE=1 cargo run
```

### Frontend
- React DevTools browser extension
- TanStack Query DevTools (included in dev)
- Network tab for GraphQL inspection

### Database
```bash
# Connect to PostgreSQL
docker compose exec postgres psql -U resonance

# Check active connections
SELECT * FROM pg_stat_activity;

# Analyze slow queries
EXPLAIN ANALYZE <query>;
```

### Redis
```bash
# Connect to Redis CLI
docker compose exec redis redis-cli

# Monitor all commands
MONITOR

# Check pub/sub channels
PUBSUB CHANNELS "user:*"
```

### WebSocket
Use browser DevTools Network tab → WS filter to inspect WebSocket messages.

---

## Performance Considerations

- **Audio streaming**: Use HTTP range requests, avoid loading entire files
- **GraphQL**: Use DataLoader pattern to avoid N+1 queries
- **Search**: Meilisearch handles full-text; pgvector for semantic
- **Caching**: Redis for sessions, hot data; PostgreSQL for persistence
- **Frontend**: React.memo for list items, virtualize large lists

---

## Security Notes

### Authentication & Tokens
- **Token Storage**: Access and refresh tokens stored in localStorage via Zustand persist
  - Trade-off: Simpler implementation for self-hosted use case
  - Risk: Tokens accessible to JavaScript (XSS vulnerable)
  - Mitigation: Short-lived access tokens (15 min), token rotation on refresh
  - Future: Consider HTTP-only cookies for public-facing deployments
- **Password Hashing**: Argon2id with memory-hard parameters
- **Timing Attack Prevention**: Constant-time password comparison

### Infrastructure Security
- Row-Level Security (RLS) in PostgreSQL per user
- Rate limiting on auth endpoints (Redis-backed with in-memory fallback)
- Trusted proxy configuration for X-Forwarded-For validation
- CORS with explicit allowed origins
- Input validation via serde (backend) and zod (frontend)
- Never log sensitive data (passwords, tokens)

### JWT Configuration
- Minimum 32-character secret required (enforced at startup)
- Access tokens: 15-minute expiry
- Refresh tokens: 7-day expiry with rotation
