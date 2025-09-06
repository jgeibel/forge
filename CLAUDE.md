# Forge - Minecraft-like MMO Project

## Project Overview
Building a voxel-based MMO game similar to Minecraft with a unique twist: all realms are planets in a shared universe. Think Minecraft meets No Man's Sky.

## Current Status
- **Phase**: Planning & Setup
- **Next Step**: Initialize Bevy project and implement basic voxel rendering
- **Target Platforms**: PC (Windows) and macOS (Apple Silicon native)

## Key Features
- Block-based voxel world
- Farming and building mechanics
- Inventory and crafting system
- Resource gathering
- Massively multiplayer with planet-based instances
- Inter-planet travel within shared universe

## Tech Stack
- **Engine**: Bevy (Rust) - ECS architecture, excellent performance
- **Graphics**: wgpu (Metal on Mac, DirectX/Vulkan on Windows)
- **Networking**: QUIC protocol via quinn-rs
- **Databases**: 
  - ScyllaDB for world/chunk data
  - PostgreSQL for player data
  - Redis for caching
- **Backend**: Kubernetes-orchestrated Rust servers
- **Serialization**: Protocol Buffers

## Documentation Structure
- `docs/ROADMAP.md` - Complete phase breakdown with progress tracking
- `docs/ARCHITECTURE.md` - Technical decisions and system design
- `docs/phases/` - Detailed documentation for each development phase
- `docs/decisions/` - Architecture Decision Records (ADRs)

## Development Phases
1. **Core Voxel Engine** (Weeks 1-4) - Basic rendering and interaction
2. **Inventory & Crafting** (Weeks 5-6) - Item system
3. **World Generation** (Weeks 7-8) - Procedural terrain
4. **Networking Foundation** (Weeks 9-12) - Multiplayer base
5. **Planet System** (Weeks 13-14) - Universe architecture
6. **Persistence Layer** (Weeks 15-16) - Save/load systems
7. **MMO Features** (Weeks 17-20) - Social and gameplay systems

## Quick Commands
```bash
# Run the game (once initialized)
cargo run

# Run tests
cargo test

# Build for release
cargo build --release

# Build for macOS
cargo build --target aarch64-apple-darwin

# Build for Windows
cargo build --target x86_64-pc-windows-msvc
```

## Important Notes for Claude
- This is a long-term project broken into phases
- Always check ROADMAP.md for current progress
- Prioritize cross-platform compatibility
- Focus on performance and scalability from the start
- Each planet is an isolated game server for better scaling