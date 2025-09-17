# Repository Guidelines

## Project Structure & Module Organization
Game logic lives in `src/`, with domain-focused modules such as `planet/`, `world/`, `chunk/`, and `inventory/` for terrain, world state, and player systems. Rendering helpers sit under `render/`, UI flows under `ui/`, and reusable tools in `tools.rs`. Runtime assets (textures, meshes, audio) reside in `assets/`; keep large test data out of Git and note the helper scripts `create_temp_textures.py` and `resize_textures.sh` for placeholder or resized textures. Architectural notes and long-form plans are in `docs/`, while `bevy_console_chat/` contains the bespoke in-game console plugin currently disabled in `Cargo.toml`.

## Build, Test, and Development Commands
Use `cargo build` for a release-quality check and `cargo run` (default target `forge`) for the desktop client. `cargo test` runs the existing unit coverage; add `-- --nocapture` when debugging output-heavy tests. For rapid iteration, install `cargo-watch` and launch `./run-hot.sh`, which rebuilds on changes to `src/`. Format the workspace with `cargo fmt` and lint before reviews with `cargo clippy --all-targets --all-features`.

## Coding Style & Naming Conventions
Follow standard Rust style: four-space indentation, `snake_case` for modules/files, `CamelCase` for types, and `SCREAMING_SNAKE_CASE` for constants. Keep systems lightweight and focused; prefer splitting large stages into dedicated modules (e.g., `loading.rs` for asset preloads). Commit formatted code (`cargo fmt`) and address clippy warnings before opening a PR.

## Testing Guidelines
Unit tests belong either in `#[cfg(test)] mod tests` blocks next to the code or under a future `tests/` directory for integration coverage. Name tests after the behavior under check (`handles_chunk_edge_cases`). Leverage Bevy's headless mode where possible and gate longer-running terrain generation tests with `#[ignore]`. Aim to cover both simulation logic (`planet/`, `world/`) and data loaders (`loading.rs`).

## Commit & Pull Request Guidelines
Recent history favors short imperative commits (`Add command prompt`, `Fix day/night cycle`); keep following that style and scope a commit per logical change. Pull requests should summarize gameplay impact, list testing commands (`cargo run`, `cargo test`), link any tracked issue, and attach screenshots or clips when you touch rendering, UI, or new assets. Coordinate breaking engine changes in `docs/ROADMAP.md` before merging.

## Assets & Tooling Notes
Version only source textures and procedural rules; generated intermediates should go to `target/` or a temporary folder. Use the provided shell/python helpers for asset prep, and document any new external dependencies in `docs/ARCHITECTURE.md` so other contributors can reproduce your setup.
