# Resonance: Self-Hosted Music Streaming Platform

> A Spotify-like self-hosted music player with AI features, Lidarr integration, and real-time cross-device sync.

## Project Summary

| Aspect | Decision |
|--------|----------|
| **Name** | Resonance |
| **Repo** | `/Users/cjvana/Documents/GitHub/resonance` |
| **Backend** | Rust + Axum |
| **Frontend** | React + TypeScript + Vite |
| **Database** | PostgreSQL 16 + pgvector |
| **Cache** | Redis 7 |
| **Search** | Meilisearch |
| **AI** | Ollama (single model, user-specified) |
| **Deployment** | Docker Compose |

---

## Core Features

### Must-Have (v1.0)
- [x] Music playback (gapless, crossfade, equalizer)
- [x] Album art-based visualizer
- [x] Multi-user with built-in auth
- [x] Real-time cross-device sync (essential)
- [x] Lidarr integration with auto-download
- [x] Spotify-like autoplay (smart prefetch overnight)
- [x] Weekly Discover playlist (scheduled, auto-downloads)
- [x] AI: Mood detection, NL search, auto-tagging, chat
- [x] Lyrics (synced + static fallback)
- [x] Playlists (basic, smart, collaborative)
- [x] Discord Rich Presence + ListenBrainz
- [x] PWA with unlimited offline caching
- [x] Smart transcoding (auto + manual quality)

---

## Tech Stack

### Backend (Rust)
```
axum          - Web framework
tokio         - Async runtime
sqlx          - PostgreSQL driver
async-graphql - GraphQL API
symphonia     - Audio decoding
redis         - Caching + pub/sub
meilisearch-sdk - Search
```

### Frontend (React)
```
React 18      - UI framework
TypeScript    - Type safety
Vite          - Build tool
TanStack Query - Server state
Zustand       - Client state
Radix UI      - Accessible components
Web Audio API - Audio processing
Workbox       - PWA/offline
```

### Infrastructure
```
PostgreSQL 16 + pgvector - Primary DB + vectors
Redis 7       - Cache, pub/sub, sessions
Meilisearch   - Fast search with typo tolerance
Ollama        - Local AI (user-specified model)
Docker        - Containerization
```

---

## Architecture

### System Overview
```
┌─────────────────────────────────────────────────────────────┐
│                        Clients                               │
│  (Web PWA, Mobile PWA, Future: React Native)                │
└──────────────────────────┬──────────────────────────────────┘
                           │
                    ┌──────▼──────┐
                    │   Traefik/  │
                    │   Cloudflare│
                    │    Tunnel   │
                    └──────┬──────┘
                           │
┌──────────────────────────▼──────────────────────────────────┐
│                    Resonance API                             │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐           │
│  │  Auth   │ │ Library │ │Playback │ │   AI    │           │
│  └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘           │
│       │           │           │           │                  │
│  ┌────▼───────────▼───────────▼───────────▼────┐            │
│  │              Core Services                   │            │
│  │  (Streaming, Sync, Recommendations, Search)  │            │
│  └──────────────────────────────────────────────┘            │
└──────────────────────────┬──────────────────────────────────┘
                           │
        ┌──────────────────┼──────────────────┐
        │                  │                  │
   ┌────▼────┐       ┌─────▼─────┐      ┌─────▼─────┐
   │PostgreSQL│       │   Redis   │      │Meilisearch│
   │+pgvector │       │           │      │           │
   └──────────┘       └───────────┘      └───────────┘
        │
   ┌────▼────┐       ┌───────────┐      ┌───────────┐
   │  Ollama │       │  Lidarr   │      │  Music    │
   │  (AI)   │       │   API     │      │  Library  │
   └─────────┘       └───────────┘      └───────────┘
```

### Real-Time Sync Architecture
```
Device A ──────┐
               │     ┌─────────────────┐
Device B ──────┼────►│  Redis Pub/Sub  │◄───► WebSocket Server
               │     │  user:{id}:*    │
Device C ──────┘     └─────────────────┘
```

---

## Database Schema (Key Tables)

### Users & Auth
- `users` - User accounts, preferences, roles
- `sessions` - JWT sessions with device info

### Library
- `artists` - Artist metadata, MusicBrainz ID, Lidarr ID
- `albums` - Album metadata, cover art colors (for visualizer)
- `tracks` - Track metadata, file info, audio features, AI tags
- `track_embeddings` - pgvector embeddings for semantic search

### User Data
- `playlists` - User playlists (manual, smart, collaborative)
- `playlist_tracks` - Playlist track ordering
- `listening_history` - Play history for recommendations
- `user_library` - Liked tracks, albums, artists

### AI & Recommendations
- `recommendation_cache` - Cached recommendations
- `pending_downloads` - Lidarr download queue

---

## Implementation Phases

### Phase 1: Foundation
1. Initialize monorepo structure
2. Set up Docker Compose with all services
3. Implement PostgreSQL schema + migrations
4. Create Rust API skeleton with Axum
5. Set up React app with Vite + TypeScript
6. Implement authentication (JWT + sessions)

### Phase 2: Library & Playback
1. Lidarr integration (sync library)
2. Audio streaming endpoint with range requests
3. On-the-fly transcoding (FFmpeg/symphonia)
4. Web Audio API player (gapless, crossfade)
5. Equalizer implementation (10-band)
6. Queue management

### Phase 3: Real-Time Features
1. WebSocket server for sync
2. Redis pub/sub for cross-device
3. Playback state synchronization
4. Device handoff (transfer playback)
5. Presence system (now playing)

### Phase 4: AI & Recommendations
1. Ollama integration
2. Audio feature extraction (Essentia)
3. Embedding generation (nomic-embed-text)
4. Mood/vibe detection
5. Natural language search
6. Chat assistant
7. Collaborative filtering recommendations
8. Content-based recommendations

### Phase 5: Advanced Features
1. Smart playlists (rule engine)
2. Collaborative playlists
3. Weekly Discover playlist generation
4. Autoplay with smart prefetch
5. Lidarr auto-download workflow
6. Lyrics (synced + static)
7. Album art visualizer

### Phase 6: Integrations & Polish
1. Discord Rich Presence
2. ListenBrainz scrobbling
3. PWA with offline support
4. User settings & preferences
5. Admin dashboard
6. Social features (optional sharing)

### Phase 7: Production
1. Documentation
2. CI/CD pipeline
3. Docker image publishing
4. Performance optimization
5. Security audit

---

## Project Structure

```
resonance/
├── apps/
│   ├── api/                    # Rust backend
│   │   ├── src/
│   │   │   ├── routes/         # HTTP/GraphQL endpoints
│   │   │   ├── services/       # Business logic
│   │   │   ├── models/         # Database models
│   │   │   ├── websocket/      # Real-time sync
│   │   │   └── graphql/        # GraphQL schema
│   │   └── Cargo.toml
│   │
│   ├── worker/                 # Background job processor
│   │   └── src/
│   │       └── jobs/           # Scheduled tasks
│   │
│   └── web/                    # React frontend
│       └── src/
│           ├── components/     # UI components
│           ├── hooks/          # Custom hooks
│           ├── stores/         # Zustand stores
│           ├── lib/            # Utilities
│           └── pages/          # Route pages
│
├── packages/
│   └── shared-types/           # Shared TypeScript types
│
├── docker/
│   ├── Dockerfile              # API
│   ├── Dockerfile.worker       # Worker
│   └── init.sql                # DB initialization
│
├── docker-compose.yml
├── .env.example
└── README.md
```

---

## Docker Compose Services

| Service | Image | Purpose |
|---------|-------|---------|
| `resonance` | Custom | Main API server |
| `resonance-worker` | Custom | Background jobs |
| `postgres` | pgvector/pgvector:pg16 | Database |
| `redis` | redis:7-alpine | Cache/pub-sub |
| `meilisearch` | getmeili/meilisearch:v1.6 | Search |
| `ollama` | ollama/ollama | Local AI |

---

## Environment Variables

```bash
# Required
DB_PASSWORD=
JWT_SECRET=
MEILISEARCH_KEY=
LIDARR_URL=
LIDARR_API_KEY=
MUSIC_LIBRARY_PATH=

# Optional
PORT=8080
LISTENBRAINZ_API_KEY=
DISCORD_CLIENT_ID=
OLLAMA_MODEL=mistral
```

---

## API Design

### GraphQL (Primary)
- Queries: Library, search, recommendations, playlists
- Mutations: Auth, playlist management, settings
- Subscriptions: Playback sync, presence

### REST (Specialized)
- `GET /stream/:trackId` - Audio streaming
- `POST /webhooks/lidarr` - Lidarr notifications
- `GET /health` - Health check

---

## Key Technical Decisions

### Why Rust?
- Zero-cost abstractions for audio processing
- No GC pauses during streaming (critical for gapless)
- Excellent async I/O with Tokio
- Strong audio libraries (symphonia)

### Why PostgreSQL + pgvector?
- Single database for all data + vectors
- ACID guarantees
- Row-level security for multi-tenant
- Proven at 200k+ track scale

### Why GraphQL?
- Flexible queries for different clients
- Real-time subscriptions built-in
- Type-safe with code generation

### Why Modular Monolith?
- Simpler deployment (single Docker image)
- No network overhead between modules
- Can extract to microservices later if needed

---

## Security

- JWT authentication with refresh tokens
- Row-Level Security in PostgreSQL
- Rate limiting per endpoint
- CORS with explicit origins
- Input validation via serde
- No secrets in code (env vars only)

---

## Offline Strategy (PWA)

1. **Service Worker**: Cache static assets + API responses
2. **IndexedDB**: Store downloaded tracks
3. **Background Sync**: Sync play history when online
4. **Cache API**: Smart caching of frequently played
5. **User Control**: Download playlists for offline

---

## Lidarr Workflow

```
User approves recommendation
         │
         ▼
┌─────────────────┐
│ Check if local  │──► Yes ──► Play immediately
└────────┬────────┘
         │ No
         ▼
┌─────────────────┐
│ Add to Lidarr   │
│ download queue  │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Notify user:    │
│ "Downloading..."│
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Play similar    │
│ available track │
└────────┬────────┘
         │
    (Webhook)
         │
         ▼
┌─────────────────┐
│ Track ready!    │
│ Add to library  │
└─────────────────┘
```

---

## Next Steps

1. **Create repository** at `/Users/cjvana/Documents/GitHub/resonance`
2. **Initialize monorepo** with Cargo workspace + pnpm
3. **Set up Docker Compose** with all services
4. **Implement Phase 1** (Foundation)

---

## Open Questions (Resolved)

| Question | Answer |
|----------|--------|
| Backend language | Rust |
| Project location | /Users/cjvana/Documents/GitHub/resonance |
| Offline storage limit | Unlimited |
| AI feature rollout | All at once (v1.0) |
| Ollama model | User will specify later |
| Mobile approach | PWA first, native later |
