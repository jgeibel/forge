# Hydrology System Redesign

## Goals & Constraints
- Replace the existing erosion-heavy simulation with a world-gen only pipeline that prioritizes clean, realistic rivers and numerous ponds.
- Maintain compatibility with the current generation order (terrain ➜ mountains ➜ climate ➜ islands ➜ hydrology) and reuse the terrain and mountain products without modifying upstream logic.
- Deliver convincing river morphology at a resolution consistent with the terrain chunking while keeping the implementation approachable and deterministic.
- Preserve existing metadata serialization (bincode/serde) and the `WorldMetadata` layout, adjusting only the hydrology payload contents.

## Inputs & Integration Points
- **Terrain Components** (`WorldGenerator::terrain_components`): provides base elevation, highlands, and mountain range accents already computed before hydrology.
- **Plate & Mountain Data**: mountain ranges and plate samples inform plausible river sources with higher precipitation and steeper slopes.
- **Climate Sampling**: rainfall noise (`raw_rainfall` and `hydrology_rain_noise`) remains available for tuning river density and pond placement.
- **World Builder**: continues to surface hydrology parameters; controls must be updated to reflect the new configuration set.
- **Sampling API**: `WorldGenerator::sample_hydrology` and downstream uses in terrain/biome phases should continue to receive water level, channel depth, river intensity, and lake (pond) intensity values.

## Proposed Pipeline
1. **Hydrology Raster Preparation**
   - Rasterize `base_height` and rainfall onto the configured hydrology grid (default square resolution).
   - Compute per-cell slope magnitude using central differences to inform flow strength and pond suitability.
   - Derive a `terrain_mask` indicating land vs ocean based on sea level and continent masks.

2. **Depression Filling & Basin Graph Construction**
   - Apply a fast priority-flood fill (e.g., Barnes et al.) to remove spurious sinks while recording basin spill elevations and volumes.
   - Output: `filled_height`, `basin_id`, `spill_elevation` per basin, and a directed acyclic graph of basin connectivity down to sea outlets.

3. **Flow Direction & Accumulation**
   - Use D8 (with wrap-around) to assign a primary flow direction per cell from `filled_height`.
   - Compute `flow_accum` via topological order over the basin DAG, seeding rainfall and optional snowpack factors; accumulation is stored both raw (cell count) and normalized by drainage area.
   - Identify `stream_order` (Strahler or Horton) to support main river promotion and width scaling.

4. **River Source Selection**
   - Choose source cells where accumulation exceeds a configurable percentile, slope is above a minimum, and rainfall bias is high (favoring mountains/interiors).
   - Snap sources to ridge lines (using slope direction and mountain range map) to avoid double-counting adjacent cells.
   - Cap the number of major sources per continental basin to avoid overcrowding and ensure “clean” primary rivers.

5. **Channel Tracing & Geometry**
   - Trace from each source down the flow network to an outlet, building polylines with sub-cell offsets to smooth the D8 stair-stepping.
   - Apply a lightweight meander model: perturb control points with band-limited noise proportional to local slope and stream order, ensuring curvature stays within realistic ranges.
   - Merge tributaries by honoring stream order; only higher-order rivers continue as explicit channels while smaller streams contribute to widening/depth.
   - Promote the strongest discharge paths into "major rivers" that are guaranteed to reach the sea; keep a configurable count and depth boost so every continent gets a few headline rivers.

6. **River Carving & Water Levels**
   - For each polyline segment, carve a parametric cross-section (parabolic or compound) scaled by accumulation to produce `channel_depth` and `bankfull_width` rasters.
   - Compute `water_level` as `min(surface_height, spill_elevation)` plus a configurable depth fraction, ensuring consistency with sea level downstream.
   - Generate a `floodplain` softness field near wide rivers for terrain phase blending.

7. **Pond Detection & Filling**
   - Revisit residual basins with low outflow (flat or enclosed) and mark those below a size threshold as ponds.
   - Determine pond water level using the basin spill elevation minus a configurable safety margin; store radius-equivalent metrics for use in terrain/biome sampling.
   - Output `pond_mask` and `pond_depth` arrays, with IDs to support biome placement (e.g., wetlands).

8. **Coastal & Estuary Treatment**
   - Blend river channels into coastal cells by tapering width and aligning water levels with sea level using a configurable estuary length.
   - Produce `coastal_factor` similar to the existing system to keep shoreline smoothing behavior intact.

## Data Products & Sampling
- Replace the current `HydrologySimulation` payload with:
  - `channel_depth`, `water_level`, `river_intensity` (normalized stream order),
  - `pond_intensity` (formerly `lake_intensity`),
  - `floodplain_softness`, `coastal_factor`,
  - optional debug layers (`flow_accum`, `stream_order`, `basin_id`) behind `#[cfg(debug_assertions)]`.
- `HydrologySample` remains but `lake_intensity` is renamed internally to `pond_intensity`; sampling logic performs bilinear filtering as today.
- `WorldGenerator::get_water_level` adjusts to respect pond depth caps and river depth scaling derived from the new arrays.

## Configuration Updates
- Deprecate erosion-related knobs (`hydrology_iterations`, `time_step`, `infiltration_rate`, etc.).
- Introduce concise parameters:
  - `hydrology_river_density` (percentile for accumulation threshold),
  - `hydrology_river_width_scale`, `hydrology_river_depth_scale`,
  - `hydrology_meander_strength`,
  - `hydrology_pond_density`, `hydrology_pond_size_range`,
  - `hydrology_estuary_length`, `hydrology_floodplain_radius`.
- Update `docs/world_builder_defaults.json` and the world builder UI to expose these new fields while hiding removed ones.

## Implementation Phases
1. **Scaffolding & Data Structures**
   - Introduce a new hydrology module implementing the pipeline above; keep the existing file until replacement is ready to avoid breaking build.
   - Define helper structs (`HydrologyRaster`, `BasinGraph`, `RiverPolyline`) and unit tests for core algorithms (priority flood, accumulation).

2. **Flow & Basin Core**
   - Implement raster prep, depression filling, and flow accumulation; validate using debug visualizations in world builder.

3. **River Network Extraction**
   - Build source selection and polyline tracing; integrate with carving to populate `channel_depth`/`river_intensity`.

4. **Pond System**
   - Detect basins classified as ponds, compute water levels, and populate pond rasters.

5. **Integration & Refactor**
   - Swap the old hydrology module with the new implementation, update config struct/serialization, adapt terrain and biome phases to renamed fields.
   - Remove obsolete erosion-specific code paths.

6. **Tooling & Docs**
   - Refresh world builder panels, document parameter meanings in `docs/ARCHITECTURE.md`, and add screenshots demonstrating river/pond output.

## Risks & Open Questions
- **Resolution Balance**: river curvature fidelity depends on hydrology grid resolution; may need adaptive sampling for very large planets.
- **Serialization Compatibility**: ensure serde defaults keep older saves loadable by providing fallback values for removed fields.
- **Performance**: although world-gen is offline, priority-flood and polyline smoothing must be optimized for large maps (potential parallelization with Rayon).
- **Biome Coupling**: confirm downstream biome logic uses only the retained fields; adjust if additional wetland indicators are desired.
