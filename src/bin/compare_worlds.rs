use forge::world::config::WorldGenConfig;
use forge::world::generator::{WorldGenerator, WorldMetadata};
use forge::world::package::planet_package_paths;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let world_name = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "Alpha".to_string());

    let (config_path, metadata_path) = planet_package_paths(&world_name);

    println!("Comparing world data for '{}'", world_name);
    println!("  Config path   : {:?}", config_path);
    println!("  Metadata path : {:?}", metadata_path);

    let config_contents = std::fs::read_to_string(&config_path)?;
    let config: WorldGenConfig = serde_json::from_str(&config_contents)?;
    println!(
        "Loaded config with seed {} and planet_size {}",
        config.seed, config.planet_size
    );

    let generator_live = WorldGenerator::with_progress(config.clone(), |_| {});
    let generator_rebuild = WorldGenerator::with_progress(config.clone(), |_| {});

    let (metadata, format) = WorldMetadata::load_from_file(&metadata_path)?;
    println!("Loaded metadata via {:?}", format);

    let generator_cached = WorldGenerator::from_metadata(metadata.clone());
    let cached_config = generator_cached.config();
    println!(
        "Cached config seed {} planet_size {}",
        cached_config.seed, cached_config.planet_size
    );

    let mut max_height_diff = 0.0_f32;
    let mut avg_height_diff = 0.0_f32;
    let mut max_water_diff = 0.0_f32;
    let mut max_moisture_diff = 0.0_f32;
    let mut max_temp_diff = 0.0_f32;

    let mut total_samples = 0u32;

    let mut max_height_diff_rebuild = 0.0_f32;

    let mut rng = StdRng::seed_from_u64(0xC0FFEE);
    for _ in 0..20 {
        let base_x = rng.gen_range(0.0..config.planet_size as f32);
        let base_z = rng.gen_range(0.0..config.planet_size as f32);
        let step = config.planet_size as f32 / 64.0;

        for dz in 0..64 {
            for dx in 0..64 {
                let world_x = (base_x + dx as f32 * step).rem_euclid(config.planet_size as f32);
                let world_z = (base_z + dz as f32 * step).rem_euclid(config.planet_size as f32);

                let height_a = generator_live.get_height(world_x, world_z);
                let height_b = generator_cached.get_height(world_x, world_z);
                let dh = (height_a - height_b).abs();
                max_height_diff = max_height_diff.max(dh);
                avg_height_diff += dh;

                let height_rebuild = generator_rebuild.get_height(world_x, world_z);
                max_height_diff_rebuild =
                    max_height_diff_rebuild.max((height_a - height_rebuild).abs());

                let water_a = generator_live.get_water_level(world_x, world_z);
                let water_b = generator_cached.get_water_level(world_x, world_z);
                max_water_diff = max_water_diff.max((water_a - water_b).abs());

                let moisture_a = generator_live.get_moisture(world_x, world_z);
                let moisture_b = generator_cached.get_moisture(world_x, world_z);
                max_moisture_diff = max_moisture_diff.max((moisture_a - moisture_b).abs());

                let temp_a = generator_live.temperature_at_height(world_x, world_z, height_a);
                let temp_b = generator_cached.temperature_at_height(world_x, world_z, height_b);
                max_temp_diff = max_temp_diff.max((temp_a - temp_b).abs());

                total_samples += 1;
            }
        }
    }

    if total_samples > 0 {
        avg_height_diff /= total_samples as f32;
    }

    println!("Samples compared: {}", total_samples);
    println!("Max height diff   : {:.6}", max_height_diff);
    println!(
        "Max height diff (fresh vs fresh) : {:.6}",
        max_height_diff_rebuild
    );
    println!("Avg height diff   : {:.6}", avg_height_diff);
    println!("Max water level diff : {:.6}", max_water_diff);
    println!("Max moisture diff : {:.6}", max_moisture_diff);
    println!("Max temperature diff : {:.6}", max_temp_diff);

    Ok(())
}
