# Repository Guidelines

## Project Structure & Module Organization
Game logic lives in `src/` with domain modules such as `planet/`, `world/`, `chunk/`, and `inventory/` handling terrain simulation, world state, and player systems. Rendering helpers reside under `src/render/`, UI flows sit in `src/ui/`, and shared utilities collect in `src/tools.rs`. Runtime assets (textures, meshes, audio) stay in `assets/`. Long-form plans and architecture notes live in `docs/`, while `bevy_console_chat/` hosts the optional in-game console plugin disabled in `Cargo.toml`. Keep generated assets out of Git; use helper scripts like `assets/create_temp_textures.py` when prototyping.

## Build, Test, and Development Commands
Run `cargo build` for a release-quality compile check, and `cargo run` (default target `forge`) to launch the desktop client. Hot-reload changes with `./run-hot.sh` once `cargo-watch` is installed. Use `cargo fmt` to format before committing and `cargo clippy --all-targets --all-features` to catch lint violations. Prefer `cargo test -- --nocapture` when you need verbose output.

## Coding Style & Naming Conventions
Follow Rust defaults: four-space indentation, `snake_case` for files and functions, `CamelCase` for types, and `SCREAMING_SNAKE_CASE` for constants. Keep systems focused; break large stages into modules such as `loading.rs` for asset preloads. Document new tooling in `docs/ARCHITECTURE.md` so teammates can reproduce your setup.

## Testing Guidelines
Write unit tests inline using `#[cfg(test)]` modules or move integration coverage under `tests/` as it grows. Name tests after the behavior under check, e.g., `handles_chunk_edge_cases`. Leverage Bevy headless mode for simulation-heavy coverage, and gate long terrain generation tests with `#[ignore]`. Run `cargo test` before opening a PR.

## Commit & Pull Request Guidelines
Use short, imperative commit messages like `Add command prompt` or `Fix day/night cycle`. PRs should summarize gameplay impact, list commands executed (`cargo run`, `cargo test`), reference any tracked issues, and attach screenshots or clips when touching rendering or UI. Coordinate breaking engine changes in `docs/ROADMAP.md` before merge.

## World Builder Tool
Spin up the standalone world builder with `cargo run --bin world_builder`. The app opens a map window plus a tabbed control panel for tuning `WorldGenConfig` (terrain, islands, and hydrology), cycling planet sizes, and re-generating previews. Click the map to inspect biome, height, water level, and other stats for a location, and use “Save as Defaults” to persist the current configuration to `docs/world_builder_defaults.json`.

## Security & Configuration Tips
Restrict large temporary assets to `target/` or scratch directories. Note any external dependencies, environment variables, or engine tweaks in `docs/ARCHITECTURE.md` so other contributors stay in sync.
