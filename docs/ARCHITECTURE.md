# Forge Architecture Documentation

## System Overview

Forge is a voxel-based MMO that combines Minecraft-style gameplay with a universe of interconnected planets. Each planet operates as an independent game server while sharing a common universe infrastructure.

```
┌─────────────────────────────────────────────────────────────┐
│                         Clients                              │
│         (Windows/Mac/Linux - Bevy + wgpu)                   │
└─────────────┬───────────────────────────┬───────────────────┘
              │                           │
              │         QUIC              │
              ▼                           ▼
┌─────────────────────────────────────────────────────────────┐
│                    Load Balancer (Envoy)                    │
└─────────────┬───────────────────────────┬───────────────────┘
              │                           │
              ▼                           ▼
┌─────────────────────────────────────────────────────────────┐
│                    Planet Servers                            │
│         (Kubernetes Pods - Bevy Headless)                   │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  │
│  │ Planet A │  │ Planet B │  │ Planet C │  │ Planet D │  │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘  │
└─────────────┬───────────────────────────┬───────────────────┘
              │                           │
              ▼                           ▼
┌─────────────────────────────────────────────────────────────┐
│                    Data Layer                                │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐            │
│  │  ScyllaDB  │  │ PostgreSQL │  │   Redis    │            │
│  │  (Chunks)  │  │  (Players) │  │  (Cache)   │            │
│  └────────────┘  └────────────┘  └────────────┘            │
└─────────────────────────────────────────────────────────────┘
```

## Technology Stack

### Core Game Engine
**Bevy (Rust)**
- **Why**: ECS architecture perfect for voxel worlds, excellent performance, growing ecosystem
- **Version**: 0.14+ (latest stable)
- **Key Features Used**:
  - Entity Component System for game objects
  - Built-in rendering pipeline
  - Asset management
  - Input handling
  - Audio system

### Graphics & Rendering
**wgpu**
- **Why**: Cross-platform GPU API, native performance, modern architecture
- **Backends**:
  - macOS: Metal
  - Windows: DirectX 12 / Vulkan
  - Linux: Vulkan
- **Techniques**:
  - Greedy meshing for voxel optimization
  - Instanced rendering for repeated blocks
  - Compute shaders for lighting/AO
  - LOD system with distance-based detail

### Terrain & Hydrology
- Terrain height comes from layered Perlin noise (continents, detail, mountains) with optional island masks.
- Hydrology uses a configurable flow-accumulation map that carves river beds, fills lakes, and feeds both chunk generation and map previews.
- Parameters (rainfall, flow thresholds, depth scaling, etc.) are surfaced in the world-builder UI for rapid iteration.
- Rainfall variance/frequency sliders plus major-river boost controls sculpt regional wet/dry belts and spawn a handful of signature rivers.

### Data Serialization
**Serde + serde_json**
- **Why**: Lightweight, battle-tested serialization for Rust structs
- **Usage**: Persisting world-generation presets (e.g., `docs/world_builder_defaults.json`) and other configuration snapshots used by tooling

### Networking
**QUIC Protocol (quinn-rs)**
- **Why**: Better than TCP for games, handles packet loss gracefully, built-in encryption
- **Architecture**:
  - Reliable ordered streams for critical data
  - Unreliable channels for position updates
  - Delta compression for state changes
  - Client-side prediction with server reconciliation

**Protocol Buffers**
- **Why**: Efficient serialization, schema evolution, cross-platform
- **Usage**: All network messages, save files, configuration

### Backend Infrastructure

**Kubernetes**
- **Why**: Automatic scaling, self-healing, rolling updates
- **Components**:
  - Planet servers as StatefulSets
  - Service mesh for inter-service communication
  - Horizontal pod autoscaling based on player count
  - Persistent volumes for world data

**Envoy Proxy**
- **Why**: Advanced load balancing, observability, resilience
- **Features**:
  - Sticky sessions for player-to-planet affinity
  - Circuit breaking for failing servers
  - Automatic retry logic
  - Detailed metrics and tracing

### Data Persistence

**ScyllaDB (Chunks & World Data)**
- **Why**: Cassandra-compatible, extreme performance, horizontal scaling
- **Schema**: 
  ```
  planet_id | chunk_x | chunk_y | chunk_z | data (blob) | version | modified_at
  ```
- **Partitioning**: By planet_id for data locality

### Chunk Payload Format

To support on-demand streaming and persistence, baked voxel chunks are packaged using
the `v1` chunk payload format:

- **Magic**: `FBCH` (4 bytes) followed by a one-byte `version` field.
- **Palette**: `u16` length + packed list of unique block IDs (`BlockType` as `u8`). A single chunk
  usually stays below 32 unique block types, keeping the palette tiny.
- **Runs**: `u32` run count + run entries (`u16` palette index, `u16` length). Runs are stored in
  X-major order and always sum to `32×32×32` voxels. This RLE compresses air-heavy chunks down to a
  few dozen bytes while staying CPU-cheap to decode.

`ChunkStorage::encode_payload` collapses raw voxel arrays into this payload, while
`ChunkStorage::from_payload` restores the in-memory layout for rendering and physics.
The format is intentionally forward-compatible: newer payload versions should increment the
version byte and keep the existing header so old servers can reject unsupported data cleanly.

To inspect payloads during development set `FORGE_DEBUG_CHUNK_PAYLOADS_DIR` (defaults to no-op)
and run the game or tools—the chunk pipeline will emit serialized blobs to that directory as
chunks bake or mutate. For a quick end-to-end check without the full client, run the harness:

```
cargo run --bin chunk_payload_debug [output_dir]
```

The binary generates a sample chunk, applies an edit, and writes the resulting payload revisions to
`target/chunk_payload_debug/` (or the provided directory). Remove the directory when you’re done to
avoid stale captures.

Runtime chunk persistence is controlled separately:

- `FORGE_PERSISTENCE_DIR` (default `target/chunk_payload_persistence/`) points the live persistence
  handler at a writable directory. This is a stub that mirrors the future planet-server writer.
- `FORGE_PERSISTENCE_ENABLED` accepts `0/false` to disable the handler while keeping the rest of the
  pipeline intact. Any other value (or absence) keeps it on.
- On startup the chunk loader now checks this directory and rehydrates the latest revision of each
  chunk before falling back to procedural baking, so edits survive restarts once persisted files are
  present.

For quick inspection of per-plate lithology, use:

```
cargo run --bin lithology_probe <world_x> <world_z> [planet_size_blocks]
```

This prints the surface block, strata thicknesses, basement type, and cave/ore bias that will be
applied when chunks at that location are baked.

**PostgreSQL (Player Data)**
- **Why**: ACID compliance, complex queries, JSONB for flexible schemas
- **Tables**:
  - players (uuid, username, email, created_at)
  - inventories (player_id, items JSONB, updated_at)
  - achievements (player_id, achievement_id, unlocked_at)
  - friends (player_id, friend_id, status)

**Redis (Cache Layer)**
- **Why**: Sub-millisecond latency, pub/sub for real-time events
- **Usage**:
  - Active chunk cache (LRU eviction)
  - Player session data
  - Planet server registry
  - Inter-planet messaging
  - Rate limiting

### Development Tools

**Build System**
- Cargo for Rust dependencies
- Docker for containerization
- Helm for Kubernetes deployments

**Testing**
- Unit tests with cargo test
- Integration tests with test containers
- Load testing with custom tools
- Network simulation for latency/packet loss

## Key Design Patterns

### Entity Component System (ECS)
```rust
// Example components
struct Position { x: f32, y: f32, z: f32 }
struct Velocity { x: f32, y: f32, z: f32 }
struct BlockType { id: u16 }
struct Inventory { items: Vec<Item> }

// Systems operate on components
fn movement_system(mut query: Query<(&mut Position, &Velocity)>) {
    for (mut pos, vel) in query.iter_mut() {
        pos.x += vel.x;
        pos.y += vel.y;
        pos.z += vel.z;
    }
}
```

### Chunk Management
- Chunks are 32x32x32 voxels
- Loaded in a radius around players
- Compressed when inactive
- Streamed on-demand
- Cached in Redis for hot access

### Network Protocol
```protobuf
message ChunkData {
    int32 x = 1;
    int32 y = 2;
    int32 z = 3;
    bytes compressed_blocks = 4;
    uint32 version = 5;
}

message PlayerUpdate {
    string player_id = 1;
    Position position = 2;
    Rotation rotation = 3;
    uint32 sequence = 4;
}
```

### Planet Isolation
- Each planet runs as independent server
- No shared state between planets
- Players transfer via session handoff
- Allows horizontal scaling
- Failure isolation

## Performance Targets

### Client Performance
- **FPS**: 60+ on mid-range hardware (GTX 1060 / M1 MacBook Air)
- **Memory**: < 4GB RAM usage
- **Load Time**: < 10 seconds to join planet
- **Render Distance**: 16+ chunks

### Server Performance
- **Players per Planet**: 100+ concurrent
- **Tick Rate**: 20 TPS minimum
- **Latency**: < 100ms for 90% of players
- **Chunk Generation**: < 50ms per chunk

### Infrastructure
- **Availability**: 99.9% uptime
- **Planet Spin-up**: < 30 seconds
- **Auto-scaling**: React within 60 seconds
- **Backup Frequency**: Every 5 minutes

## Security Considerations

### Anti-Cheat
- Server-authoritative for all actions
- Validation of all client inputs
- Rate limiting on actions
- Anomaly detection for impossible movements
- Replay system for review

### Data Protection
- TLS 1.3 for all connections (via QUIC)
- Encrypted at rest (database level)
- GDPR compliance for EU players
- Regular security audits

### Authentication
- JWT tokens for session management
- OAuth2 for third-party login
- 2FA support
- Account recovery system

## Scalability Strategy

### Vertical Scaling
- Planet servers can use up to 4 CPU cores
- Memory limit of 8GB per planet
- SSD storage for chunk data

### Horizontal Scaling
- New planets spawn automatically based on demand
- Player distribution across planets
- Geographic distribution (US, EU, Asia)
- CDN for static assets

### Database Scaling
- ScyllaDB sharding by planet_id
- Read replicas for PostgreSQL
- Redis cluster mode
- Backup to S3-compatible storage

## Development Workflow

### Local Development
```bash
# Run local server
cargo run --bin server

# Run client
cargo run --bin client

# Run with hot reload
cargo watch -x run
```

### CI/CD Pipeline
1. Push to GitHub
2. GitHub Actions runs tests
3. Build Docker images
4. Push to registry
5. Helm deployment to staging
6. Automated tests
7. Manual promotion to production

### Monitoring
- Prometheus for metrics
- Grafana for dashboards
- Jaeger for distributed tracing
- ELK stack for logs
- PagerDuty for alerts

## Platform-Specific Considerations

### macOS (Apple Silicon)
- Native ARM64 builds
- Metal rendering backend
- Unified memory advantages
- Code signing and notarization

### Windows
- DirectX 12 preferred, Vulkan fallback
- MSI installer
- Windows Defender exceptions
- Steam integration

### Future: Linux
- Vulkan only
- AppImage or Flatpak distribution
- Wayland and X11 support

## Future Architecture Considerations

### Modding Support
- WASM plugins for safety
- Resource pack system
- Server-side scripting API
- Client-side UI modifications

### Mobile Clients
- Separate rendering pipeline
- Touch controls
- Reduced chunk distance
- Simplified shaders

### Web Client
- Bevy WASM build
- WebGPU when available
- IndexedDB for caching
- WebRTC for networking
