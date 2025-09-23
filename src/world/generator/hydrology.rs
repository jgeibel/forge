use std::cmp::Ordering;

use super::util::lerp_f32;
use super::WorldGenerator;

#[derive(Clone, Copy, Default)]
pub(super) struct HydrologySample {
    pub(super) channel_depth: f32,
    pub(super) water_level: f32,
    pub(super) river_intensity: f32,
    pub(super) lake_intensity: f32,
    pub(super) rainfall: f32,
    pub(super) major_river: f32,
}

#[derive(Clone)]
pub(super) struct HydrologyMap {
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) planet_size: f32,
    pub(super) sea_level: f32,
    pub(super) river_max_depth: f32,
    pub(super) lake_depth: f32,
    pub(super) river_depth: Vec<f32>,
    pub(super) water_level: Vec<f32>,
    pub(super) river_mask: Vec<f32>,
    pub(super) lake_mask: Vec<f32>,
    pub(super) rainfall: Vec<f32>,
    pub(super) major_path: Vec<f32>,
    pub(super) major_strength: Vec<f32>,
}

impl HydrologyMap {
    pub(super) fn empty() -> Self {
        Self {
            width: 1,
            height: 1,
            planet_size: 1.0,
            sea_level: 0.0,
            river_max_depth: 0.0,
            lake_depth: 0.0,
            river_depth: vec![0.0_f32],
            water_level: vec![0.0_f32],
            river_mask: vec![0.0_f32],
            lake_mask: vec![0.0_f32],
            rainfall: vec![0.0_f32],
            major_path: vec![0.0_f32],
            major_strength: vec![0.0_f32],
        }
    }

    pub(super) fn generate(generator: &WorldGenerator) -> Self {
        let config = &generator.config;
        let width = config.hydrology_resolution.max(32) as usize;
        let height = width;
        let planet_size = config.planet_size as f32;
        let sea_level = config.sea_level;
        let count = width * height;

        let mut heights = vec![0.0_f32; count];
        let mut rainfall_map = vec![0.0_f32; count];

        for y in 0..height {
            for x in 0..width {
                let u = (x as f32 + 0.5) / width as f32;
                let v = (y as f32 + 0.5) / height as f32;
                let world_x = u * planet_size;
                let world_z = v * planet_size;
                let components = generator.terrain_components(world_x, world_z);
                let idx = y * width + x;
                heights[idx] = components.base_height;
                rainfall_map[idx] = generator.raw_rainfall(world_x, world_z);
            }
        }

        let mut downstream: Vec<Option<usize>> = vec![None; count];
        for y in 0..height {
            for x in 0..width {
                let idx = y * width + x;
                let current_height = heights[idx];
                if current_height <= sea_level {
                    continue;
                }

                let mut lowest_height = current_height;
                let mut target = None;
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let neighbor =
                            Self::wrap_index(width, height, x as isize + dx, y as isize + dy);
                        let neighbor_height = heights[neighbor];
                        if neighbor_height < lowest_height {
                            lowest_height = neighbor_height;
                            target = Some(neighbor);
                        }
                    }
                }

                downstream[idx] = target;
            }
        }

        let mut upstream: Vec<Vec<usize>> = vec![Vec::new(); count];
        for (idx, &target) in downstream.iter().enumerate() {
            if let Some(target) = target {
                upstream[target].push(idx);
            }
        }

        let mut order: Vec<usize> = (0..count).collect();
        order.sort_unstable_by(|a, b| {
            heights[*b]
                .partial_cmp(&heights[*a])
                .unwrap_or(Ordering::Equal)
        });

        let river_threshold = config.river_flow_threshold.max(0.0);
        let depth_scale = config.river_depth_scale.max(0.0);
        let max_depth = config.river_max_depth.max(0.01);
        let surface_ratio = config.river_surface_ratio.clamp(0.1, 1.0);
        let lake_threshold = config.lake_flow_threshold.max(0.0);
        let lake_depth = config.lake_depth.max(0.0);

        let mut flow_accum = vec![0.0_f32; count];
        let mut river_depth = vec![0.0_f32; count];
        let mut water_level = vec![0.0_f32; count];
        let mut river_mask = vec![0.0_f32; count];
        let mut lake_mask = vec![0.0_f32; count];
        let mut major_path_mask = vec![0.0_f32; count];
        let mut major_strength = vec![0.0_f32; count];

        if config.hydrology_major_river_count > 0 && config.hydrology_major_river_boost > 0.0 {
            // Try to generate 3x more rivers than requested, keep the best ones that reach ocean
            let attempts = (config.hydrology_major_river_count * 3).min(count as u32) as usize;
            let desired = config.hydrology_major_river_count.min(count as u32) as usize;
            if desired > 0 {
                // Find the highest points on land as river sources
                // Adaptive threshold based on actual terrain heights
                let max_height = heights.iter().cloned().fold(0.0_f32, f32::max);
                let height_range = max_height - sea_level;
                let mountain_threshold = sea_level + height_range * 0.3; // Top 70% of elevation range

                let mut land_cells: Vec<(usize, f32)> = heights
                    .iter()
                    .enumerate()
                    .filter(|(_, h)| **h > mountain_threshold)
                    .map(|(idx, height)| (idx, *height))
                    .collect();
                land_cells
                    .sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));

                let mut sources = Vec::new();
                let mut attempts_used = 0;

                for &(idx, _) in &land_cells {
                    if attempts_used >= attempts || sources.len() >= desired * 3 {
                        break;
                    }
                    attempts_used += 1;

                    // Walk downstream to see if this source reaches ocean
                    let mut current = idx;
                    let mut visited = std::collections::HashSet::new();
                    visited.insert(current);

                    let mut reached_ocean = false;
                    let mut path = vec![current];
                    while let Some(next) = downstream[current] {
                        if heights[next] <= sea_level {
                            reached_ocean = true;
                            break;
                        }
                        if !visited.insert(next) {
                            // Loop detected
                            break;
                        }
                        current = next;
                        path.push(current);
                        if path.len() > width * 4 {
                            break;
                        }
                    }

                    if reached_ocean {
                        sources.push((idx, path));
                        if sources.len() >= desired {
                            break;
                        }
                    }
                }

                for (idx, path) in sources {
                    let boost = config.hydrology_major_river_boost;
                    let strength = 1.0 + boost * 0.75;
                    major_strength[idx] = strength;
                    for (i, cell) in path.iter().enumerate() {
                        let weight = ((path.len() - i) as f32 / path.len() as f32).clamp(0.2, 1.0);
                        major_path_mask[*cell] = major_path_mask[*cell].max(weight);
                        major_strength[*cell] = major_strength[*cell].max(strength * weight);
                    }
                }
            }
        }

        for &idx in &order {
            let rainfall = rainfall_map[idx];
            let terrain_height = heights[idx];
            if terrain_height <= sea_level {
                continue;
            }

            let mut major_contribution = 0.0;
            if major_path_mask[idx] > 0.0 {
                major_contribution = config.hydrology_major_river_boost.max(0.0) * 0.8;
            }

            let mut feeder_flow = 0.0;
            for &source in &upstream[idx] {
                feeder_flow += flow_accum[source];
            }
            let water = flow_accum[idx] + rainfall + major_contribution;
            let major_scale = if major_path_mask[idx] > 0.0 {
                (1.0 + config.hydrology_major_river_boost.max(0.0)).max(1.5)
            } else {
                1.0
            };
            let local_threshold = if major_path_mask[idx] > 0.0 {
                1.0 // Major rivers always form channels
            } else {
                river_threshold
            };

            if let Some(down) = downstream[idx] {
                flow_accum[down] += water;
                if water >= local_threshold {
                    let depth = if major_path_mask[idx] > 0.0 {
                        // Major rivers carve much deeper channels
                        let major_depth =
                            max_depth * (1.0 + config.hydrology_major_river_boost * 0.5);
                        ((water * depth_scale * 2.0) * major_scale).min(major_depth)
                    } else {
                        ((water * depth_scale) * major_scale).min(max_depth)
                    };
                    river_depth[idx] = river_depth[idx].max(depth);
                    let bed = terrain_height - river_depth[idx];
                    let surface = bed + river_depth[idx] * surface_ratio;
                    water_level[idx] = water_level[idx].max(surface.max(sea_level));
                    river_mask[idx] = 1.0;
                }
            } else if water >= lake_threshold {
                river_depth[idx] = river_depth[idx].max(lake_depth);
                let bed = terrain_height - river_depth[idx];
                let surface = terrain_height.max(sea_level);
                water_level[idx] = water_level[idx].max(surface.max(bed));
                lake_mask[idx] = 1.0;
            }
        }

        Self {
            width,
            height,
            planet_size,
            sea_level,
            river_max_depth: max_depth,
            lake_depth,
            river_depth,
            water_level,
            river_mask,
            lake_mask,
            rainfall: rainfall_map,
            major_path: major_path_mask,
            major_strength,
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
            return HydrologySample::default();
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

        let mut depth = bilinear(&self.river_depth).max(0.0);
        let mut water_level = bilinear(&self.water_level);
        let river_mask = bilinear(&self.river_mask).clamp(0.0, 1.0);
        let lake_mask = bilinear(&self.lake_mask).clamp(0.0, 1.0);
        let rainfall = bilinear(&self.rainfall).max(0.0);
        let major = bilinear(&self.major_path).clamp(0.0, 1.0);
        let _major_strength = bilinear(&self.major_strength).max(0.0);

        if water_level <= 0.0 {
            water_level = self.sea_level;
        }

        let coverage = river_mask.max(lake_mask);
        depth *= coverage;
        water_level = lerp_f32(self.sea_level, water_level, coverage);

        let river_intensity = if depth > 0.01 {
            (depth / self.river_max_depth).clamp(0.0, 1.0) * river_mask
        } else {
            0.0
        };

        let lake_intensity = if depth > 0.01 {
            (depth / self.lake_depth.max(0.01)).clamp(0.0, 1.0) * lake_mask
        } else {
            0.0
        };

        HydrologySample {
            channel_depth: depth,
            water_level,
            river_intensity,
            lake_intensity,
            rainfall,
            major_river: major.max(0.0),
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
        sample.river_intensity.max(sample.lake_intensity)
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
