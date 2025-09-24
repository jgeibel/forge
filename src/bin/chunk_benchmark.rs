use forge::chunk::ChunkPos;
use forge::world::generator::WorldGenerator;
use std::time::Instant;

fn main() {
    let radius: i32 = 4; // 9x9 area
    let repeats: usize = 3;

    println!(
        "Chunk generation benchmark: radius={} repeats={}",
        radius, repeats
    );

    let generator = WorldGenerator::default();
    let mut durations = Vec::new();

    let positions: Vec<ChunkPos> = (-radius..=radius)
        .flat_map(|z| (-radius..=radius).map(move |x| ChunkPos::new(x, 2, z)))
        .collect();

    for round in 0..repeats {
        for &pos in &positions {
            let start = Instant::now();
            let _storage = generator.bake_chunk(pos);
            let elapsed = start.elapsed().as_secs_f64() * 1000.0;
            durations.push(elapsed);
        }
        println!("Completed round {}", round + 1);
    }

    durations.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let total = durations.len();
    let sum: f64 = durations.iter().sum();
    let avg = sum / total as f64;
    let median = durations[total / 2];
    let min = durations.first().copied().unwrap_or(0.0);
    let max = durations.last().copied().unwrap_or(0.0);
    let throughput = 1000.0 / avg;

    println!("Chunks generated: {}", total);
    println!(
        "Avg: {:.2} ms  Median: {:.2} ms  Min: {:.2} ms  Max: {:.2} ms",
        avg, median, min, max
    );
    println!("Throughput: {:.2} chunks/sec", throughput);
}
