pub struct AtlasBuilder {
    texture_size: u32,
    padding: u32,
    textures: Vec<image::RgbaImage>,
}

impl AtlasBuilder {
    pub fn new(texture_size: u32, padding: u32) -> Self {
        Self {
            texture_size,
            padding,
            textures: Vec::new(),
        }
    }

    pub fn add_texture(&mut self, texture: image::RgbaImage) -> usize {
        let index = self.textures.len();
        self.textures.push(texture);
        index
    }

    pub fn build(self) -> (image::RgbaImage, Vec<(f32, f32, f32, f32)>) {
        let texture_count = self.textures.len().max(1);
        let grid_size = (texture_count as f32).sqrt().ceil() as u32;
        let padded_size = self.texture_size + self.padding * 2;
        let atlas_size = grid_size * padded_size;

        let mut atlas = image::RgbaImage::new(atlas_size, atlas_size);
        let mut uvs = Vec::new();

        for (index, texture) in self.textures.iter().enumerate() {
            let grid_x = (index as u32) % grid_size;
            let grid_y = (index as u32) / grid_size;
            let x = grid_x * padded_size + self.padding;
            let y = grid_y * padded_size + self.padding;

            // Copy texture to atlas
            for dy in 0..self.texture_size.min(texture.height()) {
                for dx in 0..self.texture_size.min(texture.width()) {
                    let pixel = texture.get_pixel(dx, dy);
                    atlas.put_pixel(x + dx, y + dy, *pixel);
                }
            }

            // Add padding by extending edges
            Self::add_padding_static(&mut atlas, x, y, self.texture_size, self.padding);

            // Calculate UV coordinates
            let u_min = x as f32 / atlas_size as f32;
            let v_min = y as f32 / atlas_size as f32;
            let u_max = (x + self.texture_size) as f32 / atlas_size as f32;
            let v_max = (y + self.texture_size) as f32 / atlas_size as f32;

            uvs.push((u_min, v_min, u_max, v_max));
        }

        (atlas, uvs)
    }

    fn add_padding_static(
        atlas: &mut image::RgbaImage,
        x: u32,
        y: u32,
        texture_size: u32,
        padding: u32,
    ) {
        let size = texture_size;

        // Top and bottom padding
        for i in 0..size {
            let top_pixel = *atlas.get_pixel(x + i, y);
            let bottom_pixel = *atlas.get_pixel(x + i, y + size - 1);

            for p in 1..=padding {
                if y >= p {
                    atlas.put_pixel(x + i, y - p, top_pixel);
                }
                if y + size + p - 1 < atlas.height() {
                    atlas.put_pixel(x + i, y + size + p - 1, bottom_pixel);
                }
            }
        }

        // Left and right padding
        for i in 0..size {
            let left_pixel = *atlas.get_pixel(x, y + i);
            let right_pixel = *atlas.get_pixel(x + size - 1, y + i);

            for p in 1..=padding {
                if x >= p {
                    atlas.put_pixel(x - p, y + i, left_pixel);
                }
                if x + size + p - 1 < atlas.width() {
                    atlas.put_pixel(x + size + p - 1, y + i, right_pixel);
                }
            }
        }

        // Corner padding
        let tl = *atlas.get_pixel(x, y);
        let tr = *atlas.get_pixel(x + size - 1, y);
        let bl = *atlas.get_pixel(x, y + size - 1);
        let br = *atlas.get_pixel(x + size - 1, y + size - 1);

        for py in 1..=padding {
            for px in 1..=padding {
                // Top-left corner
                if x >= px && y >= py {
                    atlas.put_pixel(x - px, y - py, tl);
                }
                // Top-right corner
                if x + size + px - 1 < atlas.width() && y >= py {
                    atlas.put_pixel(x + size + px - 1, y - py, tr);
                }
                // Bottom-left corner
                if x >= px && y + size + py - 1 < atlas.height() {
                    atlas.put_pixel(x - px, y + size + py - 1, bl);
                }
                // Bottom-right corner
                if x + size + px - 1 < atlas.width() && y + size + py - 1 < atlas.height() {
                    atlas.put_pixel(x + size + px - 1, y + size + py - 1, br);
                }
            }
        }
    }
}
