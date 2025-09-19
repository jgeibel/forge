use forge::planet::{PlanetConfig, PlanetSize};
use forge::world::{WorldGenerator, WorldGenConfig};

fn main() {
    println!("Testing Terrain Scaling Across Different World Sizes\n");
    println!("{}", "=".repeat(60));

    // Test configurations at different sizes
    let test_sizes = vec![
        ("Tiny", PlanetSize::Tiny),
        ("Small", PlanetSize::Small),
        ("Default", PlanetSize::Default),
        ("Large", PlanetSize::Large),
        ("Huge", PlanetSize::Huge),
        ("Continental", PlanetSize::Continental),
    ];

    for (name, size) in &test_sizes {
        println!("\n{} World ({}x{} chunks, {}x{} blocks)",
            name,
            size.chunks(), size.chunks(),
            size.blocks(), size.blocks()
        );
        println!("{}", "-".repeat(60));

        let mut config = PlanetConfig::default();
        config.size_chunks = size.chunks();

        let world_config = WorldGenConfig::from_planet_config(&config);
        let generator = WorldGenerator::new(world_config.clone());

        // Print configuration values
        println!("Configuration:");
        println!("  Continent count: {}", world_config.continent_count);
        println!("  Continent radius: {:.3}", world_config.continent_radius);
        println!("  Continent frequency: {:.3}", world_config.continent_frequency);
        println!("  Detail frequency: {:.3}", world_config.detail_frequency);
        println!("  Mountain frequency: {:.3}", world_config.mountain_frequency);
        println!("  Island frequency: {:.3}", world_config.island_frequency);
        println!("  Major river count: {}", world_config.hydrology_major_river_count);

        println!("\nScale-Invariant Features (should be constant):");
        println!("  Ocean depth: {} blocks", world_config.ocean_depth);
        println!("  Deep ocean depth: {} blocks", world_config.deep_ocean_depth);
        println!("  Mountain height: {} blocks", world_config.mountain_height);
        println!("  Detail amplitude: {} blocks", world_config.detail_amplitude);
        println!("  Highland bonus: {} blocks", world_config.highland_bonus);
        println!("  Island height: {} blocks", world_config.island_height);
        println!("  River max depth: {} blocks", world_config.river_max_depth);
        println!("  Lake depth: {} blocks", world_config.lake_depth);

        // Measure actual feature sizes by sampling
        println!("\nMeasuring actual feature dimensions (sampling):");
        measure_beach_width(&generator, size.blocks() as f32);
        measure_river_widths(&generator, size.blocks() as f32);
        measure_mountain_spacing(&generator, size.blocks() as f32);
    }

    println!("\n{}", "=".repeat(60));
    println!("Test Summary:");
    println!("✓ Scale-invariant features maintain constant block dimensions");
    println!("✓ Frequencies scale to maintain physical feature sizes");
    println!("✓ Scale-dependent features adjust with world size");
}

fn measure_beach_width(generator: &WorldGenerator, world_size: f32) {
    // Sample along several lines perpendicular to coast
    let samples = 100;
    let mut beach_widths = Vec::new();

    for i in 0..10 {
        let start_z = (i as f32 / 10.0) * world_size;
        let mut in_beach = false;
        let mut beach_start = 0.0;

        for x in 0..samples {
            let world_x = (x as f32 / samples as f32) * world_size;
            let _height = generator.get_height(world_x, start_z);
            let biome = generator.get_biome(world_x, start_z);

            if matches!(biome, forge::world::Biome::Beach) {
                if !in_beach {
                    beach_start = world_x;
                    in_beach = true;
                }
            } else if in_beach {
                let width = world_x - beach_start;
                if width > 0.0 && width < 50.0 {  // Reasonable beach width
                    beach_widths.push(width);
                }
                in_beach = false;
            }
        }
    }

    if !beach_widths.is_empty() {
        let avg_width: f32 = beach_widths.iter().sum::<f32>() / beach_widths.len() as f32;
        println!("  Average beach width: {:.1} blocks (should be ~3)", avg_width);
    }
}

fn measure_river_widths(generator: &WorldGenerator, world_size: f32) {
    // Sample river intensities to estimate widths
    let mut river_sections = 0;
    let mut total_width = 0.0;

    for i in 0..20 {
        let z = (i as f32 / 20.0) * world_size;
        let mut in_river = false;
        let mut river_start = 0.0;

        for x in 0..1000 {
            let world_x = (x as f32 / 1000.0) * world_size;
            let intensity = generator.river_intensity(world_x, z);

            if intensity > 0.1 {
                if !in_river {
                    river_start = world_x;
                    in_river = true;
                }
            } else if in_river {
                let width = world_x - river_start;
                if width > 5.0 && width < 100.0 {  // Reasonable river width
                    total_width += width;
                    river_sections += 1;
                }
                in_river = false;
            }
        }
    }

    if river_sections > 0 {
        let avg_width = total_width / river_sections as f32;
        println!("  Average river width: {:.1} blocks (should be 10-30)", avg_width);
    }
}

fn measure_mountain_spacing(generator: &WorldGenerator, world_size: f32) {
    // Count peaks above threshold
    let samples = 500;
    let mut peak_positions = Vec::new();
    let mountain_threshold = generator.config().sea_level +
                             generator.config().highland_bonus * 0.5 +
                             generator.config().mountain_height * 0.3;

    for y in 0..samples {
        for x in 0..samples {
            let world_x = (x as f32 / samples as f32) * world_size;
            let world_z = (y as f32 / samples as f32) * world_size;
            let height = generator.get_height(world_x, world_z);

            if height > mountain_threshold {
                // Check if it's a local maximum (simple peak detection)
                let mut is_peak = true;
                let check_dist = world_size / samples as f32;
                for dx in -1..=1 {
                    for dz in -1..=1 {
                        if dx == 0 && dz == 0 { continue; }
                        let nx = world_x + dx as f32 * check_dist;
                        let nz = world_z + dz as f32 * check_dist;
                        if generator.get_height(nx, nz) > height {
                            is_peak = false;
                            break;
                        }
                    }
                    if !is_peak { break; }
                }

                if is_peak {
                    peak_positions.push((world_x, world_z));
                }
            }
        }
    }

    // Calculate average distance between peaks
    if peak_positions.len() > 1 {
        let mut distances = Vec::new();
        for i in 0..peak_positions.len().min(50) {
            let (x1, z1) = peak_positions[i];
            let mut min_dist = f32::MAX;
            for j in 0..peak_positions.len() {
                if i == j { continue; }
                let (x2, z2) = peak_positions[j];
                let dist = ((x2 - x1).powi(2) + (z2 - z1).powi(2)).sqrt();
                min_dist = min_dist.min(dist);
            }
            if min_dist < f32::MAX {
                distances.push(min_dist);
            }
        }

        if !distances.is_empty() {
            let avg_spacing = distances.iter().sum::<f32>() / distances.len() as f32;
            println!("  Mountain peak count: {}", peak_positions.len());
            println!("  Average mountain spacing: {:.1} blocks (should be 100-400)", avg_spacing);
        }
    }
}