use std::env;

use forge::world::{WorldGenConfig, WorldGenerator};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: lithology_probe <world_x> <world_z> [planet_size_blocks]");
        std::process::exit(1);
    }

    let world_x: f32 = args[1].parse().expect("world_x must be a number (blocks)");
    let world_z: f32 = args[2].parse().expect("world_z must be a number (blocks)");

    let mut config = WorldGenConfig::default();
    if let Some(size_arg) = args.get(3) {
        config.planet_size = size_arg
            .parse()
            .expect("planet_size must be an integer number of blocks");
    }

    println!(
        "Using planet size {} blocks, seed {}",
        config.planet_size, config.seed
    );

    let generator = WorldGenerator::new(config.clone());
    let profile = generator.lithology_profile_at(world_x, world_z);
    let height = generator.get_height(world_x, world_z);
    let water = generator.get_water_level(world_x, world_z);
    let hydro = generator.hydrology_debug_sample(world_x, world_z);

    println!("Location: ({:.2}, {:.2})", world_x, world_z);
    println!("Surface elevation: {:.2} (water {:.2})", height, water);
    println!(
        "Hydrology -> base {:.2}, channel {:.2}, river {:.2}, pond {:.2}, coastal {:.2}, major {:.2}",
        hydro.base_height,
        hydro.channel_depth,
        hydro.river_intensity,
        hydro.pond_intensity,
        hydro.coastal_factor,
        generator.major_river_factor(world_x, world_z)
    );
    println!(
        "Surface block: {:?} (depth {} blocks)",
        profile.surface_block, profile.surface_depth
    );

    let mut cumulative = profile.surface_depth as i32;
    for (idx, layer) in profile.strata.iter().enumerate() {
        cumulative += layer.thickness as i32;
        println!(
            "Layer {}: {:?} thickness {} (cumulative depth {})",
            idx + 1,
            layer.block,
            layer.thickness,
            cumulative
        );
    }

    println!(
        "Basement block below depth {}: {:?}",
        cumulative, profile.basement_block
    );
    println!(
        "Cave bias {:.2}, ore bias {:.2}",
        profile.cave_bias, profile.ore_bias
    );
}
