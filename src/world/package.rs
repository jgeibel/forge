use std::path::PathBuf;

const BASE_DIR: &str = "assets/worlds";

pub fn planet_package_paths(world_name: &str) -> (PathBuf, PathBuf) {
    let base = PathBuf::from(BASE_DIR).join(world_name);
    let config = base.join("planet.json");
    let metadata = base.join("metadata.bin");
    (config, metadata)
}
