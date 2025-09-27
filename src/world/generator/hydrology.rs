use bevy::log::info;
use noise::NoiseFn;
use serde::{Deserialize, Serialize};
use std::cmp::{Ordering, Reverse};
use std::collections::{BinaryHeap, VecDeque};

use super::util::lerp_f32;
use super::WorldGenerator;
use crate::world::config::WorldGenConfig;
use crate::world::defaults;

const NEIGHBORS: [(isize, isize); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

#[derive(Clone, Copy, Default, Serialize, Deserialize)]
pub(crate) struct HydrologySample {
    pub(super) channel_depth: f32,
    pub(super) water_level: f32,
    pub(super) river_intensity: f32,
    pub(super) pond_intensity: f32,
    pub(super) rainfall: f32,
    pub(super) major_river: f32,
    pub(super) coastal_factor: f32,
}

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct HydrologySimulation {
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) planet_size: f32,
    pub(super) sea_level: f32,
    pub(super) rainfall: Vec<f32>,
    pub(super) base_height: Vec<f32>,
    pub(super) filled_height: Vec<f32>,
    pub(super) channel_depth: Vec<f32>,
    pub(super) water_level: Vec<f32>,
    pub(super) river_intensity: Vec<f32>,
    pub(super) pond_intensity: Vec<f32>,
    pub(super) major_flow: Vec<f32>,
    pub(super) coastal_factor: Vec<f32>,
    pub(super) rainfall_peak: f32,
}

impl Default for HydrologySimulation {
    fn default() -> Self {
        Self::empty()
    }
}

#[derive(Copy, Clone, PartialEq, PartialOrd)]
struct FloatOrd(f32);

impl Eq for FloatOrd {}

impl Ord for FloatOrd {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl HydrologySimulation {
    pub(super) fn empty() -> Self {
        Self {
            width: 0,
            height: 0,
            planet_size: 1.0,
            sea_level: 0.0,
            rainfall: Vec::new(),
            base_height: Vec::new(),
            filled_height: Vec::new(),
            channel_depth: Vec::new(),
            water_level: Vec::new(),
            river_intensity: Vec::new(),
            pond_intensity: Vec::new(),
            major_flow: Vec::new(),
            coastal_factor: Vec::new(),
            rainfall_peak: 0.0,
        }
    }

    pub(super) fn generate(generator: &WorldGenerator) -> Self {
        let config = &generator.config;
        let width = config.hydrology_resolution.max(1) as usize;
        let height = width;
        let planet_size = config.planet_size as f32;
        let sea_level = config.sea_level;
        let count = width * height;

        if count == 0 {
            return Self::empty();
        }

        let cell_size = (planet_size / width as f32).max(1.0);

        let mut base_height = vec![0.0_f32; count];
        let mut rainfall = vec![0.0_f32; count];

        for y in 0..height {
            for x in 0..width {
                let u = (x as f32 + 0.5) / width as f32;
                let v = (y as f32 + 0.5) / height as f32;
                let world_x = u * planet_size;
                let world_z = v * planet_size;
                let idx = y * width + x;
                let components = generator.terrain_components(world_x, world_z);
                base_height[idx] = components.base_height;
                rainfall[idx] = generator.raw_rainfall(world_x, world_z).max(0.0);
            }
        }

        let filled_height = priority_fill(&base_height, width, height, sea_level);
        let rainfall_sum: f32 = rainfall.iter().copied().sum();
        let rainfall_avg = if count > 0 {
            rainfall_sum / count as f32
        } else {
            0.0
        };
        let baseline_rainfall = defaults::HYDROLOGY_RAINFALL.max(0.001);
        let rainfall_factor = if rainfall_avg <= 0.0 || config.hydrology_rainfall <= 0.0 {
            0.0
        } else {
            (rainfall_avg / baseline_rainfall).clamp(0.01, 6.0)
        };
        let (downstream, slope_to_downstream) = compute_flow_directions(
            &filled_height,
            &base_height,
            width,
            height,
            sea_level,
            cell_size,
        );
        let flow_accum = compute_flow_accumulation(&filled_height, &downstream, &rainfall);

        let rainfall_peak = rainfall.iter().copied().fold(0.0_f32, |acc, v| acc.max(v));
        let max_flow = flow_accum
            .iter()
            .copied()
            .fold(0.0_f32, |acc, v| acc.max(v))
            .max(1.0);

        let major_count = config.hydrology_major_river_count.min(64) as usize;
        let major_min_flow_factor = config.hydrology_major_river_min_flow.clamp(0.0, 1.0);
        let major_depth_boost = config.hydrology_major_river_depth_boost.max(0.0);

        let mut upstream = vec![Vec::<usize>::new(); count];
        for idx in 0..count {
            let down = downstream[idx];
            if down != usize::MAX && down != idx {
                upstream[down].push(idx);
            }
        }

        let mut major_weight = vec![0.0_f32; count];

        let mut river_threshold = percentile_for_land(
            &flow_accum,
            &base_height,
            sea_level,
            (1.0 - config.hydrology_river_density.clamp(0.01, 0.95)).clamp(0.0, 1.0),
        );
        let mut pond_threshold = percentile_for_land(
            &flow_accum,
            &base_height,
            sea_level,
            config.hydrology_pond_density.clamp(0.01, 0.95),
        );

        if rainfall_factor <= 0.02 {
            river_threshold = f32::MAX;
            pond_threshold = f32::MAX;
        } else {
            let river_scale = rainfall_factor.powf(0.8).max(0.25);
            let pond_scale = rainfall_factor.powf(0.6).max(0.25);
            river_threshold = (river_threshold / river_scale).max(0.0);
            pond_threshold = (pond_threshold / pond_scale).max(0.0);
        }

        #[cfg(debug_assertions)]
        let mut debug_major_stats = MajorRiverStats::default();

        if major_count > 0 && rainfall_factor > 0.05 {
            let flow_cutoff = (max_flow * major_min_flow_factor).max(max_flow * 0.003);
            let mut candidates = Vec::new();
            for idx in 0..count {
                if base_height[idx] <= sea_level {
                    continue;
                }
                let down = downstream[idx];
                if down == usize::MAX {
                    continue;
                }
                if base_height[down] > sea_level + 2.0 && filled_height[down] > sea_level + 2.0 {
                    continue;
                }
                let flow = flow_accum[idx];
                if flow < flow_cutoff {
                    continue;
                }
                candidates.push((flow, idx));
            }

            if candidates.is_empty() {
                candidates = downstream
                    .iter()
                    .enumerate()
                    .filter(|&(idx, down)| {
                        *down != usize::MAX
                            && base_height[idx] > sea_level
                            && (base_height[*down] <= sea_level + 2.0
                                || filled_height[*down] <= sea_level + 2.0)
                    })
                    .map(|(idx, _)| (flow_accum[idx], idx))
                    .collect();
            }

            #[cfg(debug_assertions)]
            {
                debug_major_stats.candidate_count = candidates.len();
            }

            candidates.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(Ordering::Equal));

            #[cfg(debug_assertions)]
            {
                debug_major_stats.target_rivers = major_count;
            }

            let mut seen = vec![false; count];
            let mut traced = 0usize;
            let mut selected = Vec::new();

            for &(flow, idx) in &candidates {
                if traced >= major_count {
                    break;
                }
                if flow <= 0.0 {
                    continue;
                }

                let path = trace_centerline(
                    idx,
                    &downstream,
                    &upstream,
                    &flow_accum,
                    &base_height,
                    sea_level,
                    width,
                    height,
                    generator,
                    max_flow,
                    &seen,
                );

                if path.len() < 3 {
                    continue;
                }

                let mut overlaps = false;
                for &other in &selected {
                    if overlap_distance(idx, other, width, height) < width as f32 * 0.02 {
                        overlaps = true;
                        break;
                    }
                }
                if overlaps {
                    continue;
                }

                for &cell in &path {
                    seen[cell] = true;
                }
                selected.push(idx);

                stamp_major_weights(
                    &mut major_weight,
                    &path,
                    &flow_accum,
                    max_flow,
                    config,
                    width,
                    height,
                );

                #[cfg(debug_assertions)]
                {
                    debug_major_stats.centerlines += 1;
                    debug_major_stats.touched_cells += path.len();
                }

                traced += 1;
            }

            #[cfg(debug_assertions)]
            {
                debug_major_stats.spaced_count = selected.len();
            }
        }

        #[cfg(debug_assertions)]
        {
            info!(
                "major rivers: target={} traced={} touched={} spaced={} candidates={} fallback={}",
                debug_major_stats.target_rivers,
                debug_major_stats.centerlines,
                debug_major_stats.touched_cells,
                debug_major_stats.spaced_count,
                debug_major_stats.candidate_count,
                debug_major_stats.fallback_used
            );
        }

        let mut channel_depth = vec![0.0_f32; count];
        let mut water_level = base_height.clone();
        let mut river_intensity = vec![0.0_f32; count];
        let mut pond_intensity = vec![0.0_f32; count];
        let mut major_flow = vec![0.0_f32; count];

        for idx in 0..count {
            let base = base_height[idx];
            if base <= sea_level {
                water_level[idx] = sea_level;
                continue;
            }

            if rainfall_factor <= 0.02 {
                water_level[idx] = base.max(sea_level);
                continue;
            }

            let flow = flow_accum[idx];
            let flow_norm = (flow / max_flow).clamp(0.0, 1.0);
            let slope = slope_to_downstream[idx].max(0.00005);

            let major_strength = major_weight[idx];

            if (flow > river_threshold && flow > pond_threshold && river_threshold.is_finite())
                || major_strength > 0.0
            {
                let slope_term = (slope * 120.0).clamp(0.15, 1.3);
                let meander_term = (1.0 + config.hydrology_meander_strength * 0.15).clamp(1.0, 1.3);
                let mut depth =
                    flow_norm.powf(0.62) * config.hydrology_river_depth_scale * slope_term;
                depth = depth.max(1.0).min(config.hydrology_river_depth_scale * 1.6);
                depth *= meander_term;
                depth *= rainfall_factor.powf(0.35).clamp(0.5, 2.5);
                if major_strength > 0.0 {
                    depth += major_depth_boost * major_strength;
                }
                depth = depth.min(config.hydrology_river_depth_scale * 2.2);

                channel_depth[idx] = depth;
                let bed = base - depth;
                let desired_fill = (depth * 0.6).clamp(0.4, depth - 0.3);
                let surface = (bed + desired_fill)
                    .max(sea_level)
                    .min(filled_height[idx])
                    .min(base - 0.25);
                water_level[idx] = water_level[idx]
                    .max(sea_level)
                    .min(surface)
                    .min(base - 0.18);

                let intensity = ((flow - river_threshold)
                    / (max_flow - river_threshold).max(0.001))
                .clamp(0.0, 1.0);
                let intensity_scale = rainfall_factor.powf(0.5).clamp(0.4, 2.2);
                let scaled_intensity = (intensity * intensity_scale).clamp(0.0, 1.0);
                if major_strength > 0.0 {
                    river_intensity[idx] = scaled_intensity.max(0.75 + 0.25 * major_strength);
                    major_flow[idx] = major_strength.max(major_flow[idx]);
                } else {
                    river_intensity[idx] = scaled_intensity;
                    major_flow[idx] = scaled_intensity;
                }
            } else {
                let pond_depth = (filled_height[idx] - base).max(0.0);
                let quiet_flow = flow <= pond_threshold * 1.1;
                if pond_depth > 0.6 && quiet_flow {
                    let radius_scale = config
                        .hydrology_pond_max_radius
                        .max(config.hydrology_pond_min_radius)
                        .max(1.0);
                    let pond_scale = rainfall_factor.powf(0.45).clamp(0.3, 2.5);
                    let intensity = (pond_depth / (radius_scale * 0.18)).clamp(0.0, 1.0)
                        * (1.0 - flow_norm).clamp(0.0, 1.0)
                        * pond_scale;
                    pond_intensity[idx] = intensity.clamp(0.0, 1.0);
                    water_level[idx] = (base + pond_depth)
                        .min(filled_height[idx])
                        .max(sea_level + 0.1);
                } else {
                    water_level[idx] = base.max(sea_level);
                }
            }
        }

        let coastal_factor = compute_coastal_factor(
            &base_height,
            width,
            height,
            sea_level,
            cell_size,
            config.hydrology_estuary_length,
            config.hydrology_coastal_blend,
        );

        Self {
            width,
            height,
            planet_size,
            sea_level,
            rainfall,
            base_height,
            filled_height,
            channel_depth,
            water_level,
            river_intensity,
            pond_intensity,
            major_flow,
            coastal_factor,
            rainfall_peak,
        }
    }

    fn wrap_index(width: usize, height: usize, x: isize, y: isize) -> usize {
        let w = width as isize;
        let h = height as isize;
        let ix = ((x % w) + w) % w;
        let iy = ((y % h) + h) % h;
        (iy as usize) * width + ix as usize
    }

    pub(super) fn sample(&self, world_x: f32, world_z: f32) -> HydrologySample {
        if self.width == 0 || self.height == 0 {
            return HydrologySample {
                water_level: self.sea_level,
                ..HydrologySample::default()
            };
        }

        let u = (world_x / self.planet_size).rem_euclid(1.0);
        let v = (world_z / self.planet_size).rem_euclid(1.0);

        let fx = u * self.width as f32;
        let fy = v * self.height as f32;

        let x0 = fx.floor() as isize;
        let y0 = fy.floor() as isize;
        let tx = fx - x0 as f32;
        let ty = fy - y0 as f32;

        let bilinear = |values: &[f32]| {
            let v00 = values[Self::wrap_index(self.width, self.height, x0, y0)];
            let v10 = values[Self::wrap_index(self.width, self.height, x0 + 1, y0)];
            let v01 = values[Self::wrap_index(self.width, self.height, x0, y0 + 1)];
            let v11 = values[Self::wrap_index(self.width, self.height, x0 + 1, y0 + 1)];
            lerp_f32(lerp_f32(v00, v10, tx), lerp_f32(v01, v11, tx), ty)
        };

        let channel_depth = bilinear(&self.channel_depth).max(0.0);
        let mut water_level = bilinear(&self.water_level);
        let mut river_intensity = bilinear(&self.river_intensity).clamp(0.0, 1.0);
        let mut pond_intensity = bilinear(&self.pond_intensity).clamp(0.0, 1.0);
        let rainfall = bilinear(&self.rainfall).max(0.0);
        let major = bilinear(&self.major_flow).clamp(0.0, 1.0);
        let coastal = bilinear(&self.coastal_factor).clamp(0.0, 1.0);

        if water_level <= 0.0 {
            water_level = self.sea_level;
        }

        if river_intensity < 0.01 {
            river_intensity = 0.0;
        }
        if pond_intensity < 0.01 {
            pond_intensity = 0.0;
        }

        let rainfall_bias = (self.rainfall_peak * 0.4 + 0.3).max(0.3);
        let rain_ratio =
            ((rainfall + rainfall_bias) / (self.rainfall_peak + rainfall_bias)).clamp(0.0, 1.0);
        let rain_mix = 0.3 + 0.7 * rain_ratio.powf(1.05);
        water_level = self.sea_level + (water_level - self.sea_level) * rain_mix;
        river_intensity *= rain_mix;
        pond_intensity *= rain_mix;

        HydrologySample {
            channel_depth,
            water_level,
            river_intensity,
            pond_intensity,
            rainfall,
            major_river: (major * rain_mix).clamp(0.0, 1.0),
            coastal_factor: coastal,
        }
    }
}

impl super::WorldGenerator {
    pub fn get_water_level(&self, world_x: f32, world_z: f32) -> f32 {
        let components = self.terrain_components(world_x, world_z);
        let sample = self.sample_hydrology(world_x, world_z, components.base_height);
        let mut water_level = sample.water_level.max(self.config.sea_level);
        let height = self.get_height(world_x, world_z);

        if sample.pond_intensity > 0.05 {
            let bed_height = height;
            let pond_depth = (water_level - bed_height).max(0.0);
            let max_depth = (self.config.hydrology_pond_max_radius * 0.2).clamp(1.5, 6.0);
            let min_depth = self.config.hydrology_pond_min_radius * 0.05;
            let desired_depth = pond_depth.clamp(min_depth, max_depth);
            water_level = (bed_height + desired_depth).min(water_level);
        } else if sample.river_intensity > 0.05 {
            let bed_height = height;
            let depth_scale = self.config.hydrology_river_depth_scale.max(1.0);
            let max_depth = (depth_scale * 0.4).clamp(1.0, depth_scale);
            let min_depth = (0.3 + sample.river_intensity * 0.7).clamp(0.4, max_depth);
            let desired_depth = sample.channel_depth.clamp(min_depth, max_depth);
            water_level = (bed_height + desired_depth).min(water_level);
        }

        water_level.max(self.config.sea_level)
    }

    pub fn river_intensity(&self, world_x: f32, world_z: f32) -> f32 {
        let components = self.terrain_components(world_x, world_z);
        let sample = self.sample_hydrology(world_x, world_z, components.base_height);
        if sample.pond_intensity > 0.0 {
            sample.pond_intensity
        } else {
            sample.river_intensity
        }
    }

    pub fn major_river_factor(&self, world_x: f32, world_z: f32) -> f32 {
        let components = self.terrain_components(world_x, world_z);
        let sample = self.sample_hydrology(world_x, world_z, components.base_height);
        sample.major_river
    }

    pub fn rainfall_intensity(&self, world_x: f32, world_z: f32) -> f32 {
        let sample = self.hydrology.sample(world_x, world_z);
        if self.hydrology.width <= 1 || self.hydrology.height <= 1 {
            self.raw_rainfall(world_x, world_z)
        } else {
            sample.rainfall
        }
    }
}

fn compute_flow_directions(
    filled_height: &[f32],
    base_height: &[f32],
    width: usize,
    height: usize,
    sea_level: f32,
    cell_size: f32,
) -> (Vec<usize>, Vec<f32>) {
    let count = filled_height.len();
    let mut downstream = vec![usize::MAX; count];
    let mut slopes = vec![0.0_f32; count];

    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            if base_height[idx] <= sea_level {
                continue;
            }

            let here = filled_height[idx];
            let mut best_idx = idx;
            let mut best_height = here;
            let mut best_distance = cell_size;

            for &(dx, dy) in &NEIGHBORS {
                let neighbor = HydrologySimulation::wrap_index(
                    width,
                    height,
                    x as isize + dx,
                    y as isize + dy,
                );
                let neighbor_height = filled_height[neighbor];
                if neighbor_height > here {
                    continue;
                }
                if neighbor_height < best_height
                    || (neighbor_height == best_height && neighbor < best_idx)
                {
                    best_height = neighbor_height;
                    best_idx = neighbor;
                    best_distance = (((dx * dx + dy * dy) as f32).sqrt()).max(1.0) * cell_size;
                }
            }

            if best_idx == idx {
                for &(dx, dy) in &NEIGHBORS {
                    let neighbor = HydrologySimulation::wrap_index(
                        width,
                        height,
                        x as isize + dx,
                        y as isize + dy,
                    );
                    if base_height[neighbor] <= sea_level {
                        best_idx = neighbor;
                        best_height = sea_level;
                        best_distance = (((dx * dx + dy * dy) as f32).sqrt()).max(1.0) * cell_size;
                        break;
                    }
                }
            }

            if best_idx != idx {
                downstream[idx] = best_idx;
                let drop = (here - best_height).max(0.05);
                slopes[idx] = (drop / best_distance).max(0.00005);
            }
        }
    }

    (downstream, slopes)
}

fn compute_flow_accumulation(
    filled_height: &[f32],
    downstream: &[usize],
    rainfall: &[f32],
) -> Vec<f32> {
    let mut order: Vec<usize> = (0..filled_height.len()).collect();
    order.sort_unstable_by(|a, b| {
        filled_height[*b]
            .partial_cmp(&filled_height[*a])
            .unwrap_or(Ordering::Equal)
    });

    let mut flow = vec![0.0_f32; filled_height.len()];
    for &idx in &order {
        let rain_base = 1.0 + rainfall[idx].max(0.0) * 0.8;
        flow[idx] += rain_base;
        let downstream_idx = downstream[idx];
        if downstream_idx != usize::MAX && downstream_idx != idx {
            flow[downstream_idx] += flow[idx];
        }
    }

    flow
}

fn percentile_for_land(
    values: &[f32],
    base_height: &[f32],
    sea_level: f32,
    percentile: f32,
) -> f32 {
    let mut filtered: Vec<f32> = values
        .iter()
        .zip(base_height)
        .filter_map(|(value, &height)| {
            if height > sea_level {
                Some(*value)
            } else {
                None
            }
        })
        .collect();

    if filtered.is_empty() {
        return 0.0;
    }

    filtered.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let clamped = percentile.clamp(0.0, 1.0);
    let index = ((filtered.len() - 1) as f32 * clamped).round() as usize;
    filtered[index]
}

fn trace_centerline(
    start: usize,
    downstream: &[usize],
    upstream: &[Vec<usize>],
    flow_accum: &[f32],
    base_height: &[f32],
    sea_level: f32,
    width: usize,
    height: usize,
    generator: &WorldGenerator,
    max_flow: f32,
    seen: &[bool],
) -> Vec<usize> {
    let mut path = Vec::new();
    let mut current = start;
    let mut visited = vec![false; flow_accum.len()];
    let max_steps = (width.max(height) * 4).max(64);
    let mut steps = 0;

    while !visited[current] && !seen[current] && steps < max_steps {
        visited[current] = true;
        path.push(current);

        if base_height[current] <= sea_level + 1.0 {
            break;
        }

        let mut best_candidate = None;
        let mut best_flow = 0.0_f32;
        let mut alt_candidate = None;
        let mut alt_flow = 0.0_f32;

        for &candidate in &upstream[current] {
            let candidate_flow = flow_accum[candidate];
            if candidate_flow <= flow_accum[current] * 0.12 {
                continue;
            }
            if candidate_flow > best_flow {
                alt_candidate = best_candidate;
                alt_flow = best_flow;
                best_candidate = Some(candidate);
                best_flow = candidate_flow;
            } else if candidate_flow > alt_flow {
                alt_candidate = Some(candidate);
                alt_flow = candidate_flow;
            }
        }

        let mut next = match best_candidate {
            Some(idx) => idx,
            None => break,
        };

        if let Some(alt_idx) = alt_candidate {
            let ratio = alt_flow / best_flow.max(1e-6);
            if ratio > 0.82 {
                let (u, v) = cell_uv(alt_idx, width, height);
                let sample = generator.hydrology_rain_noise.get([
                    u * 3.173,
                    v * 3.173,
                    (max_flow as f64).fract(),
                ]);
                if sample as f32 > 0.18 {
                    next = alt_idx;
                }
            }
        }

        if downstream[next] == next {
            break;
        }

        current = next;
        steps += 1;
    }

    path
}

fn stamp_major_weights(
    weights: &mut [f32],
    path: &[usize],
    flow_accum: &[f32],
    max_flow: f32,
    config: &WorldGenConfig,
    width: usize,
    height: usize,
) {
    for &cell in path {
        let flow_norm = (flow_accum[cell] / max_flow).clamp(0.0, 1.0);
        let base_width = (config.hydrology_river_width_scale * (0.8 + flow_norm.powf(0.5) * 2.0)
            + 1.2)
            .clamp(1.5, 8.0);
        let radius = base_width;
        let radius_i = radius.ceil() as isize;
        let cx = (cell % width) as isize;
        let cy = (cell / width) as isize;

        for dy in -radius_i..=radius_i {
            for dx in -radius_i..=radius_i {
                let dist = ((dx * dx + dy * dy) as f32).sqrt();
                if dist > radius {
                    continue;
                }
                let neighbor = HydrologySimulation::wrap_index(width, height, cx + dx, cy + dy);
                let weight = (1.0 - dist / radius).powf(1.6).clamp(0.0, 1.0);
                if weight <= 0.0 {
                    continue;
                }
                weights[neighbor] = weights[neighbor].max(weight);
            }
        }
    }
}

fn cell_uv(idx: usize, width: usize, height: usize) -> (f64, f64) {
    let x = (idx % width) as f64 + 0.5;
    let y = (idx / width) as f64 + 0.5;
    (x / width as f64, y / height as f64)
}

fn overlap_distance(a: usize, b: usize, width: usize, height: usize) -> f32 {
    let ax = (a % width) as f32;
    let ay = (a / width) as f32;
    let bx = (b % width) as f32;
    let by = (b / width) as f32;
    let dx = (ax - bx).abs().min(width as f32 - (ax - bx).abs());
    let dy = (ay - by).abs().min(height as f32 - (ay - by).abs());
    dx.hypot(dy)
}

fn compute_coastal_factor(
    base_height: &[f32],
    width: usize,
    height: usize,
    sea_level: f32,
    cell_size: f32,
    estuary_length: f32,
    coastal_blend: f32,
) -> Vec<f32> {
    let count = base_height.len();
    if count == 0 || estuary_length <= 0.0 || coastal_blend <= 0.0 {
        return vec![0.0_f32; count];
    }

    let mut distance = vec![f32::INFINITY; count];
    let mut heap: BinaryHeap<Reverse<(FloatOrd, usize)>> = BinaryHeap::new();

    for idx in 0..count {
        if base_height[idx] <= sea_level {
            distance[idx] = 0.0;
            heap.push(Reverse((FloatOrd(0.0), idx)));
        }
    }

    let max_distance = estuary_length.max(cell_size);

    while let Some(Reverse((FloatOrd(dist), idx))) = heap.pop() {
        if dist > distance[idx] {
            continue;
        }
        if dist > max_distance {
            continue;
        }

        let x = (idx % width) as isize;
        let y = (idx / width) as isize;
        for &(dx, dy) in &NEIGHBORS {
            let neighbor = HydrologySimulation::wrap_index(width, height, x + dx, y + dy);
            let step = (((dx * dx + dy * dy) as f32).sqrt()).max(1.0) * cell_size;
            let next_dist = dist + step;
            if next_dist < distance[neighbor] {
                distance[neighbor] = next_dist;
                heap.push(Reverse((FloatOrd(next_dist), neighbor)));
            }
        }
    }

    distance
        .into_iter()
        .map(|dist| {
            if dist.is_infinite() {
                0.0
            } else {
                let factor = ((max_distance - dist) / max_distance).clamp(0.0, 1.0);
                (factor * coastal_blend).clamp(0.0, 1.0)
            }
        })
        .collect()
}

fn priority_fill(base_height: &[f32], width: usize, height: usize, sea_level: f32) -> Vec<f32> {
    let count = base_height.len();
    let mut filled = base_height.to_vec();
    let mut visited = vec![false; count];
    let mut heap: BinaryHeap<Reverse<(FloatOrd, usize)>> = BinaryHeap::with_capacity(count);

    for (idx, &height) in filled.iter().enumerate() {
        heap.push(Reverse((FloatOrd(height.min(sea_level + 5000.0)), idx)));
    }

    while let Some(Reverse((FloatOrd(current_height), idx))) = heap.pop() {
        if visited[idx] {
            continue;
        }
        visited[idx] = true;

        let x = (idx % width) as isize;
        let y = (idx / width) as isize;
        for &(dx, dy) in &NEIGHBORS {
            let neighbor = HydrologySimulation::wrap_index(width, height, x + dx, y + dy);
            if visited[neighbor] {
                continue;
            }
            let mut candidate = filled[neighbor];
            if candidate < current_height {
                candidate = current_height;
                filled[neighbor] = candidate;
            }
            heap.push(Reverse((FloatOrd(candidate), neighbor)));
        }
    }

    filled
}

#[cfg(debug_assertions)]
#[derive(Default)]
struct MajorRiverStats {
    target_rivers: usize,
    centerlines: usize,
    touched_cells: usize,
    spaced_count: usize,
    candidate_count: usize,
    fallback_used: bool,
}

fn label_land_components(
    base_height: &[f32],
    sea_level: f32,
    width: usize,
    height: usize,
) -> (Vec<i32>, Vec<usize>) {
    let mut component = vec![-1_i32; base_height.len()];
    let mut sizes = Vec::new();
    let mut queue = VecDeque::new();
    let mut current_id = 0_i32;

    for idx in 0..base_height.len() {
        if component[idx] != -1 || base_height[idx] <= sea_level {
            continue;
        }

        component[idx] = current_id;
        queue.push_back(idx);
        let mut size = 0usize;

        while let Some(cell) = queue.pop_front() {
            size += 1;
            let x = (cell % width) as isize;
            let y = (cell / width) as isize;
            for &(dx, dy) in &NEIGHBORS {
                let neighbor = HydrologySimulation::wrap_index(width, height, x + dx, y + dy);
                if component[neighbor] != -1 || base_height[neighbor] <= sea_level {
                    continue;
                }
                component[neighbor] = current_id;
                queue.push_back(neighbor);
            }
        }

        sizes.push(size);
        current_id += 1;
    }

    (component, sizes)
}
