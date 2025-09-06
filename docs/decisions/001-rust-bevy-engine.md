# ADR-001: Use Rust with Bevy for Game Engine

## Status
Accepted

## Context
We need to choose a game engine and programming language for building a Minecraft-like MMO that will run on both Windows and macOS (including Apple Silicon). The engine needs to handle voxel rendering efficiently, support networking, and scale to MMO requirements.

## Decision
We will use Rust as the programming language with Bevy as the game engine.

## Consequences

### Positive
- **Performance**: Rust provides memory safety without garbage collection, crucial for consistent frame rates
- **Cross-platform**: Single codebase compiles natively to Windows, macOS (including ARM64), and Linux
- **ECS Architecture**: Bevy's Entity Component System is ideal for managing thousands of game objects
- **Modern Rendering**: wgpu backend provides access to modern GPU features (Metal, DX12, Vulkan)
- **Growing Ecosystem**: Active community with voxel-specific crates emerging
- **Async Support**: Excellent for networking and I/O operations
- **Type Safety**: Catches many bugs at compile time
- **No Runtime**: Ships as single executable, no VM or runtime required

### Negative
- **Learning Curve**: Rust has steeper learning curve than traditional game dev languages
- **Compile Times**: Longer than languages like C++ or Go
- **Ecosystem Maturity**: Fewer game-specific libraries than Unity/Unreal
- **Tooling**: Debugging tools less mature than traditional game engines
- **Hiring**: Smaller pool of Rust game developers

## Alternatives Considered

### Unity with C#
- ✅ Mature ecosystem
- ✅ Large community
- ❌ Performance overhead from C#/Mono
- ❌ Licensing costs at scale
- ❌ Less control over low-level optimizations

### Unreal Engine with C++
- ✅ AAA-proven technology
- ✅ Excellent rendering
- ❌ Overkill for voxel graphics
- ❌ 5% revenue share
- ❌ Large binary size
- ❌ Complex for indie MMO

### Godot with GDScript/C++
- ✅ Open source
- ✅ Good 3D support in v4
- ❌ Less proven for MMO scale
- ❌ Would need significant voxel extensions
- ❌ GDScript performance concerns

### Custom Engine in C++
- ✅ Maximum control
- ✅ Proven performance
- ❌ Enormous development time
- ❌ Need to implement everything from scratch
- ❌ Memory safety concerns

### Go with Ebiten/G3N
- ✅ Simple language
- ✅ Good networking
- ❌ Limited 3D game libraries
- ❌ GC pauses problematic for games
- ❌ Less GPU control

## Validation
- Bevy can render 100k+ cubes at 60 FPS
- Multiple successful voxel projects exist (Veloren uses similar stack)
- WGPU proven in production (Firefox uses it)
- Rust used in performance-critical systems (Discord, Cloudflare)

## References
- [Bevy Engine](https://bevyengine.org/)
- [Are We Game Yet?](https://arewegameyet.rs/)
- [Veloren (Rust Voxel MMO)](https://veloren.net/)
- [wgpu Performance](https://github.com/gfx-rs/wgpu)