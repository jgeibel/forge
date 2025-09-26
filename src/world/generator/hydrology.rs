use serde::{Deserialize, Serialize};
use std::cmp::{Ordering, Reverse};
use std::collections::{BinaryHeap, VecDeque};

use super::util::lerp_f32;
use super::WorldGenerator;

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
#[allow(dead_code)]
pub(crate) struct HydrologySample {
    pub(super) channel_depth: f32,
    pub(super) water_level: f32,
    pub(super) river_intensity: f32,
    pub(super) lake_intensity: f32,
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
    #[allow(dead_code)]
    pub(super) base_height: Vec<f32>,
    #[allow(dead_code)]
    pub(super) filled_height: Vec<f32>,
    pub(super) channel_depth: Vec<f32>,
    pub(super) water_level: Vec<f32>,
    pub(super) river_intensity: Vec<f32>,
    pub(super) lake_intensity: Vec<f32>,
    #[allow(dead_code)]
    pub(super) discharge: Vec<f32>,
    pub(super) major_flow: Vec<f32>,
    pub(super) coastal_factor: Vec<f32>,
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
            lake_intensity: Vec::new(),
            discharge: Vec::new(),
            major_flow: Vec::new(),
            coastal_factor: Vec::new(),
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

        let cell_size = (planet_size / width as f32).max(1.0);
        let cell_area = cell_size * cell_size;
        let min_slope = config.hydrology_minimum_slope.max(0.0001);

        let terrain_slope = compute_terrain_slope(&base_height, width, height, cell_size);

        let mut flow_targets = vec![[0usize; 8]; count];
        let mut flow_weights = vec![[0.0_f32; 8]; count];
        let mut flow_counts = vec![0u8; count];

        for y in 0..height {
            for x in 0..width {
                let idx = y * width + x;
                let mut slopes = [0.0_f32; 8];
                let mut weight_total = 0.0_f32;
                for (i, (dx, dy)) in NEIGHBORS.iter().enumerate() {
                    let neighbor =
                        Self::wrap_index(width, height, x as isize + dx, y as isize + dy);
                    let height_here = filled_height[idx];
                    let height_neighbor = filled_height[neighbor];
                    if height_here <= height_neighbor {
                        continue;
                    }
                    let distance = (((dx * dx + dy * dy) as f32).sqrt()).max(1.0) * cell_size;
                    let mut drop = height_here - height_neighbor;
                    let min_drop = min_slope * distance;
                    if drop < min_drop {
                        drop = min_drop;
                    }
                    let weight = (drop / distance).powf(1.25);
                    slopes[i] = weight;
                    weight_total += weight;
                }

                if weight_total > 0.0 {
                    let mut count_out = 0;
                    for (i, weight) in slopes.iter().enumerate() {
                        if *weight <= 0.0 {
                            continue;
                        }
                        let (dx, dy) = NEIGHBORS[i];
                        let neighbor =
                            Self::wrap_index(width, height, x as isize + dx, y as isize + dy);
                        flow_targets[idx][count_out] = neighbor;
                        flow_weights[idx][count_out] = *weight / weight_total;
                        count_out += 1;
                    }
                    flow_counts[idx] = count_out as u8;
                }
            }
        }

        let mut order: Vec<usize> = (0..count).collect();
        order.sort_unstable_by(|a, b| {
            filled_height[*b]
                .partial_cmp(&filled_height[*a])
                .unwrap_or(Ordering::Equal)
        });

        let run_hydrology = config.hydrology_iterations > 0 && config.hydrology_time_step > 0.0;

        let (discharge, mut channel_depth, max_discharge, max_depth) = if run_hydrology {
            let infiltration = compute_infiltration(
                &base_height,
                &filled_height,
                &terrain_slope,
                config.hydrology_infiltration_rate,
                config.hydrology_bankfull_depth,
            );

            let dt = config.hydrology_time_step.max(0.01);
            let baseflow = (config.hydrology_baseflow * cell_area).max(0.0);

            let mut runoff = vec![0.0_f32; count];
            for idx in 0..count {
                let effective_rain = rainfall[idx] * (1.0 - infiltration[idx]);
                runoff[idx] = effective_rain * cell_area * dt + baseflow * dt;
            }

            let mut discharge = vec![0.0_f32; count];
            for &idx in &order {
                discharge[idx] += runoff[idx];
                let out_count = flow_counts[idx] as usize;
                if out_count == 0 {
                    continue;
                }
                let flow_out = discharge[idx];
                for i in 0..out_count {
                    let target = flow_targets[idx][i];
                    let weight = flow_weights[idx][i];
                    discharge[target] += flow_out * weight;
                }
            }

            let iterations = config.hydrology_iterations as usize;
            let coastal_smoothing_iters = config.hydrology_shoreline_smoothing.min(8) as usize;
            let mut channel_depth = vec![0.0_f32; count];
            let erosion_coastal_mask = compute_coastal_factor(
                &base_height,
                &filled_height,
                &channel_depth,
                width,
                height,
                sea_level,
                cell_size,
                config.hydrology_shoreline_radius,
                config.hydrology_shoreline_max_height,
                config.hydrology_shoreline_smoothing,
            );
            let mut sediment_current = vec![0.0_f32; count];
            let mut sediment_next = vec![0.0_f32; count];

            for _ in 0..iterations {
                sediment_next.fill(0.0);
                for &idx in &order {
                    let mut load = sediment_current[idx];
                    let q = discharge[idx];
                    if q <= 0.0 {
                        continue;
                    }

                    let bed = base_height[idx] - channel_depth[idx];
                    let out_count = flow_counts[idx] as usize;

                    let mut downstream_bed = bed - min_slope * cell_size;
                    if out_count > 0 {
                        let mut weighted = 0.0_f32;
                        let mut weight_sum = 0.0_f32;
                        for i in 0..out_count {
                            let target = flow_targets[idx][i];
                            let weight = flow_weights[idx][i];
                            let target_bed = base_height[target] - channel_depth[target];
                            weighted += target_bed * weight;
                            weight_sum += weight;
                        }
                        if weight_sum > 0.0 {
                            downstream_bed = weighted / weight_sum;
                        }
                    }

                    let mut slope = ((bed - downstream_bed) / cell_size).max(min_slope);
                    if !slope.is_finite() {
                        slope = min_slope;
                    }

                    let q_specific = (q / cell_area).max(0.0);
                    let stream_power = q_specific.powf(0.6) * slope.powf(1.2);
                    let capacity = (config.hydrology_sediment_capacity
                        * q_specific.powf(0.7)
                        * slope.powf(0.9))
                    .max(0.0);

                    let coastal_guard = erosion_coastal_mask[idx];
                    if coastal_guard < 0.35 && stream_power > 0.0 {
                        if load < capacity {
                            let deficit = capacity - load;
                            let erosion = stream_power * config.hydrology_erosion_rate * dt;
                            let erode = erosion.min(deficit);
                            if erode > 0.0 {
                                channel_depth[idx] = (channel_depth[idx] + erode)
                                    .min(config.hydrology_bankfull_depth);
                                load += erode;
                            }
                        } else if load > capacity {
                            let excess = load - capacity;
                            let deposit = (excess * config.hydrology_deposition_rate * dt)
                                .min(channel_depth[idx]);
                            if deposit > 0.0 {
                                channel_depth[idx] = (channel_depth[idx] - deposit).max(0.0);
                                load -= deposit;
                            }
                        }
                    }

                    if out_count == 0 {
                        let deposit = (load * config.hydrology_deposition_rate * dt)
                            .min(channel_depth[idx] * (1.0 - coastal_guard * 0.8));
                        if deposit > 0.0 {
                            channel_depth[idx] = (channel_depth[idx] - deposit).max(0.0);
                        }
                    } else {
                        for i in 0..out_count {
                            let target = flow_targets[idx][i];
                            let weight = flow_weights[idx][i];
                            sediment_next[target] += load * weight;
                        }
                    }
                }
                sediment_current.copy_from_slice(&sediment_next);

                if coastal_smoothing_iters > 0 {
                    shoreline_relax(
                        &mut channel_depth,
                        width,
                        height,
                        &erosion_coastal_mask,
                        coastal_smoothing_iters,
                    );
                }
            }

            let max_discharge = discharge
                .iter()
                .copied()
                .fold(0.0_f32, |acc, v| acc.max(v))
                .max(1.0);
            let max_depth = channel_depth
                .iter()
                .copied()
                .fold(0.0_f32, |acc, v| acc.max(v))
                .max(config.hydrology_bankfull_depth.max(1.0));

            (discharge, channel_depth, max_discharge, max_depth)
        } else {
            (vec![0.0_f32; count], vec![0.0_f32; count], 1.0, 1.0)
        };

        let mut water_level = vec![sea_level; count];
        let mut river_intensity = vec![0.0_f32; count];
        let mut lake_intensity = vec![0.0_f32; count];
        let mut major_flow = vec![0.0_f32; count];

        let coastal_factor = compute_coastal_factor(
            &base_height,
            &filled_height,
            &channel_depth,
            width,
            height,
            sea_level,
            cell_size,
            config.hydrology_shoreline_radius,
            config.hydrology_shoreline_max_height,
            config.hydrology_shoreline_smoothing,
        );

        for idx in 0..count {
            let out_count = flow_counts[idx] as usize;
            let mut depth = channel_depth[idx];
            let bed = base_height[idx] - depth;
            let fill = filled_height[idx].max(sea_level);
            let lake_depth = (fill - bed).max(0.0);
            let discharge_norm = (discharge[idx] / max_discharge).clamp(0.0, 1.0);
            let depth_ratio = (depth / max_depth).clamp(0.0, 1.0);

            let is_sink = out_count == 0;
            let is_lake = is_sink && lake_depth > 1.0;

            if is_lake {
                depth = depth.max(lake_depth);
                lake_intensity[idx] =
                    (lake_depth / (config.hydrology_bankfull_depth + 1.0)).clamp(0.0, 1.0);
                river_intensity[idx] = 0.0;
                water_level[idx] = fill;
                major_flow[idx] = 0.0;
            } else {
                let q_factor = discharge_norm.powf(0.4);
                let has_channel = discharge_norm > 0.008 || depth_ratio > 0.02;
                if has_channel {
                    let surface_ratio = 0.35 + 0.45 * depth_ratio;
                    let surface = (bed + depth * surface_ratio).max(sea_level);
                    water_level[idx] = surface.min(fill);
                    river_intensity[idx] = ((q_factor * 0.6) + (depth_ratio * 0.4)).clamp(0.0, 1.0);
                    lake_intensity[idx] = 0.0;
                    major_flow[idx] = discharge_norm.powf(0.45);
                } else {
                    // Dry land should sit at sea level so chunk baking doesn't flood mountains.
                    water_level[idx] = sea_level;
                    river_intensity[idx] = 0.0;
                    lake_intensity[idx] = 0.0;
                    major_flow[idx] = 0.0;
                    depth *= 0.5;
                }
            }

            let coastal = coastal_factor[idx];
            if coastal > 0.05 {
                depth = 0.0;
                channel_depth[idx] = 0.0;
                river_intensity[idx] = 0.0;
                lake_intensity[idx] = 0.0;
                major_flow[idx] = 0.0;
                water_level[idx] = sea_level;
            }

            channel_depth[idx] = depth;
        }

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
            lake_intensity,
            discharge,
            major_flow,
            coastal_factor,
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
        let mut lake_intensity = bilinear(&self.lake_intensity).clamp(0.0, 1.0);
        let rainfall = bilinear(&self.rainfall).max(0.0);
        let major = bilinear(&self.major_flow).clamp(0.0, 1.0);
        let coastal = bilinear(&self.coastal_factor).clamp(0.0, 1.0);

        if water_level <= 0.0 {
            water_level = self.sea_level;
        }

        if river_intensity < 0.01 {
            river_intensity = 0.0;
        }
        if lake_intensity < 0.01 {
            lake_intensity = 0.0;
        }

        HydrologySample {
            channel_depth,
            water_level,
            river_intensity,
            lake_intensity,
            rainfall,
            major_river: major,
            coastal_factor: coastal,
        }
    }
}

impl super::WorldGenerator {
    pub fn get_water_level(&self, world_x: f32, world_z: f32) -> f32 {
        let components = self.terrain_components(world_x, world_z);
        let sample = self.sample_hydrology(world_x, world_z, components.base_height);
        if sample.water_level > self.config.sea_level {
            sample.water_level
        } else if components.base_height <= self.config.sea_level {
            self.config.sea_level
        } else {
            self.config.sea_level
        }
    }

    pub fn river_intensity(&self, world_x: f32, world_z: f32) -> f32 {
        let components = self.terrain_components(world_x, world_z);
        let sample = self.sample_hydrology(world_x, world_z, components.base_height);
        if sample.lake_intensity > 0.0 {
            sample.lake_intensity
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
        for (dx, dy) in NEIGHBORS {
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

fn compute_terrain_slope(
    base_height: &[f32],
    width: usize,
    height: usize,
    cell_size: f32,
) -> Vec<f32> {
    let mut slopes = vec![0.0_f32; base_height.len()];

    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            let mut sum = 0.0_f32;
            let mut weight = 0.0_f32;
            for (dx, dy) in NEIGHBORS {
                let neighbor = HydrologySimulation::wrap_index(
                    width,
                    height,
                    x as isize + dx,
                    y as isize + dy,
                );
                let distance = (((dx * dx + dy * dy) as f32).sqrt()).max(1.0) * cell_size;
                let diff = (base_height[idx] - base_height[neighbor]).abs();
                sum += diff / distance;
                weight += 1.0;
            }
            slopes[idx] = if weight > 0.0 { sum / weight } else { 0.0 };
        }
    }

    slopes
}

fn compute_infiltration(
    base_height: &[f32],
    filled_height: &[f32],
    terrain_slope: &[f32],
    infiltration_rate: f32,
    bankfull_depth: f32,
) -> Vec<f32> {
    let mut infiltration = vec![0.0_f32; base_height.len()];

    for idx in 0..base_height.len() {
        let slope = terrain_slope[idx];
        let slope_factor = slope / (slope + 1.0);
        let ponding = ((filled_height[idx] - base_height[idx]).max(0.0)
            / (bankfull_depth * 2.0 + 1.0))
            .clamp(0.0, 1.0);
        let infil = infiltration_rate * (1.0 - slope_factor) * (1.0 - ponding * 0.75);
        infiltration[idx] = infil.clamp(0.0, 0.95);
    }

    infiltration
}

fn compute_coastal_factor(
    base_height: &[f32],
    filled_height: &[f32],
    channel_depth: &[f32],
    width: usize,
    height: usize,
    sea_level: f32,
    cell_size: f32,
    radius_world: f32,
    max_height: f32,
    smoothing_iterations: u32,
) -> Vec<f32> {
    let count = base_height.len();
    if radius_world <= 0.0 || count == 0 {
        return vec![0.0; count];
    }

    let max_height = max_height.max(0.0);
    let max_distance = radius_world.max(cell_size);
    let mut distance = vec![f32::MAX; count];
    let mut queue = VecDeque::new();

    for idx in 0..count {
        if base_height[idx] <= sea_level {
            distance[idx] = 0.0;
            queue.push_back(idx);
        }
    }

    if queue.is_empty() {
        return vec![0.0; count];
    }

    while let Some(idx) = queue.pop_front() {
        let current = distance[idx];
        if current > max_distance {
            continue;
        }
        let x = (idx % width) as isize;
        let y = (idx / width) as isize;
        for &(dx, dy) in &NEIGHBORS {
            let neighbor = HydrologySimulation::wrap_index(width, height, x + dx, y + dy);
            if distance[neighbor] <= current {
                continue;
            }
            let land_height = base_height[neighbor] - sea_level;
            if land_height > max_height {
                continue;
            }
            let step = cell_size * ((dx * dx + dy * dy) as f32).sqrt().max(1.0);
            let next = current + step;
            if next < distance[neighbor] && next <= max_distance {
                distance[neighbor] = next;
                queue.push_back(neighbor);
            }
        }
    }

    let mut coastal = vec![0.0_f32; count];
    for idx in 0..count {
        if filled_height[idx] <= sea_level {
            continue;
        }
        let d = distance[idx];
        if !d.is_finite() || d > max_distance {
            continue;
        }
        let proximity = 1.0 - (d / max_distance).clamp(0.0, 1.0);
        let elevation = if max_height > 0.0 {
            (max_height - (base_height[idx] - sea_level)).max(0.0) / max_height
        } else {
            1.0
        };
        let channel_penalty = (channel_depth[idx] / (max_height + 1.0)).min(1.0);
        coastal[idx] = (proximity * elevation * (1.0 - channel_penalty * 0.7)).clamp(0.0, 1.0);
    }

    let iterations = smoothing_iterations.min(8) as usize;
    if iterations == 0 {
        return coastal;
    }

    let mut current = coastal;
    let mut temp = vec![0.0_f32; count];
    for _ in 0..iterations {
        for idx in 0..count {
            let value = current[idx];
            if value <= 0.0 {
                temp[idx] = 0.0;
                continue;
            }
            let x = (idx % width) as isize;
            let y = (idx / width) as isize;
            let mut sum = value;
            let mut weight = 1.0;
            for &(dx, dy) in &NEIGHBORS {
                let neighbor = HydrologySimulation::wrap_index(width, height, x + dx, y + dy);
                let neighbor_value = current[neighbor];
                if neighbor_value <= 0.0 {
                    continue;
                }
                let w = if dx == 0 || dy == 0 { 1.0 } else { 0.7071 };
                sum += neighbor_value * w;
                weight += w;
            }
            temp[idx] = sum / weight;
        }
        std::mem::swap(&mut current, &mut temp);
    }

    current
}

fn shoreline_relax(
    channel_depth: &mut [f32],
    width: usize,
    height: usize,
    coastal_factor: &[f32],
    iterations: usize,
) {
    if iterations == 0 {
        return;
    }

    let mut temp = vec![0.0_f32; channel_depth.len()];
    let iterations = iterations.min(4);

    for _ in 0..iterations {
        for y in 0..height {
            for x in 0..width {
                let idx = y * width + x;
                let factor = coastal_factor[idx];
                if factor <= 0.05 {
                    temp[idx] = channel_depth[idx];
                    continue;
                }

                let mut sum = channel_depth[idx];
                let mut weight = 1.0;
                for &(dx, dy) in &NEIGHBORS {
                    let neighbor = HydrologySimulation::wrap_index(
                        width,
                        height,
                        x as isize + dx,
                        y as isize + dy,
                    );
                    let nf = coastal_factor[neighbor];
                    if nf <= 0.05 {
                        continue;
                    }
                    let w = if dx == 0 || dy == 0 { 1.0 } else { 0.7071 };
                    sum += channel_depth[neighbor] * w;
                    weight += w;
                }
                temp[idx] = sum / weight;
            }
        }

        for idx in 0..channel_depth.len() {
            if coastal_factor[idx] > 0.05 {
                channel_depth[idx] = temp[idx] * 0.6;
            }
        }
    }
}
