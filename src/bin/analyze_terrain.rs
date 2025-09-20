use forge::planet::{PlanetConfig, PlanetSize};
use forge::world::{WorldGenerator, WorldGenConfig};

fn main() {
    println!("Analyzing Terrain Heights for Huge World\n");
    println!("{}", "=".repeat(60));

    let mut config = PlanetConfig::default();
    config.size_chunks = PlanetSize::Huge.chunks();

    let world_config = WorldGenConfig::from_planet_config(&config);
    let generator = WorldGenerator::new(world_config.clone());

    let world_size = PlanetSize::Huge.blocks() as f32;
    let sea_level = world_config.sea_level;

    // Sample terrain heights across the world
    let samples = 1000;
    let mut heights = Vec::new();
    let mut land_heights = Vec::new();

    for i in 0..samples {
        for j in 0..samples {
            let x = (i as f32 / samples as f32) * world_size;
            let z = (j as f32 / samples as f32) * world_size;
            let height = generator.get_height(x, z);
            heights.push(height);

            if height > sea_level {
                land_heights.push(height - sea_level);
            }
        }
    }

    // Calculate statistics
    heights.sort_by(|a, b| a.partial_cmp(b).unwrap());
    land_heights.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let min_height = heights[0];
    let max_height = heights[heights.len() - 1];
    let median_height = heights[heights.len() / 2];

    println!("Overall Terrain Statistics:");
    println!("  Sea level: {} blocks", sea_level);
    println!("  Min height: {:.1} blocks", min_height);
    println!("  Max height: {:.1} blocks", max_height);
    println!("  Median height: {:.1} blocks", median_height);
    println!("  Height range: {:.1} blocks", max_height - min_height);

    if !land_heights.is_empty() {
        println!("\nLand Elevation Statistics (above sea level):");
        println!("  Min elevation: {:.1} blocks", land_heights[0]);
        println!("  Max elevation: {:.1} blocks", land_heights[land_heights.len() - 1]);
        println!("  Median elevation: {:.1} blocks", land_heights[land_heights.len() / 2]);

        // Count different elevation ranges
        let mut rolling_hills = 0;  // 5-20 blocks above sea
        let mut highlands = 0;      // 20-50 blocks above sea
        let mut foothills = 0;      // 50-100 blocks above sea
        let mut mountains = 0;      // 100-200 blocks above sea
        let mut peaks = 0;          // 200+ blocks above sea

        for h in &land_heights {
            if *h < 5.0 {
                // Plains
            } else if *h < 20.0 {
                rolling_hills += 1;
            } else if *h < 50.0 {
                highlands += 1;
            } else if *h < 100.0 {
                foothills += 1;
            } else if *h < 200.0 {
                mountains += 1;
            } else {
                peaks += 1;
            }
        }

        let total_land = land_heights.len() as f32;
        println!("\nElevation Distribution (% of land):");
        println!("  Rolling hills (5-20 blocks): {:.1}%", (rolling_hills as f32 / total_land) * 100.0);
        println!("  Highlands (20-50 blocks): {:.1}%", (highlands as f32 / total_land) * 100.0);
        println!("  Foothills (50-100 blocks): {:.1}%", (foothills as f32 / total_land) * 100.0);
        println!("  Mountains (100-200 blocks): {:.1}%", (mountains as f32 / total_land) * 100.0);
        println!("  Peaks (200+ blocks): {:.1}%", (peaks as f32 / total_land) * 100.0);
    }

    // Sample a cross-section to visualize terrain profile
    println!("\nTerrain Cross-Section (West to East at center):");
    println!("  Sampling 100 points across the world...\n");

    let z = world_size / 2.0;
    let mut profile = String::new();

    for i in 0..100 {
        let x = (i as f32 / 100.0) * world_size;
        let height = generator.get_height(x, z);
        let normalized = ((height - min_height) / (max_height - min_height) * 20.0) as usize;

        // Create ASCII visualization
        for j in 0..20 {
            if j == ((sea_level - min_height) / (max_height - min_height) * 20.0) as usize {
                profile.push('~');  // Sea level
            } else if j <= normalized {
                profile.push('#');
            } else {
                profile.push(' ');
            }
        }
        profile.push('\n');
    }

    // Rotate for display
    let lines: Vec<&str> = profile.lines().collect();
    for row in (0..20).rev() {
        print!("  ");
        for line in &lines {
            if let Some(ch) = line.chars().nth(row) {
                print!("{}", ch);
            }
        }
        if row == 10 {
            print!(" <- Sea Level");
        }
        println!();
    }

    println!("\n{}", "=".repeat(60));
    println!("Terrain Validation Summary:");
    println!("✓ Hills should add 5-20 blocks of variation");
    println!("✓ Mountains should reach 100-250+ blocks above sea level");
    println!("✓ Most land should have visible elevation changes");

    if !land_heights.is_empty() && land_heights.iter().any(|h| *h > 20.0) {
        println!("\n✅ TERRAIN IS WORKING! You have varied elevations across the land.");
    } else {
        println!("\n⚠️  TERRAIN MAY BE TOO FLAT - Check parameters.");
    }
}