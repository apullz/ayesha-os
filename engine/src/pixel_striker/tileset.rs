/// Tileset management for cyberpunk game maps (stub for future use).
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct Tileset {
    pub tile_width: u32,
    pub tile_height: u32,
}

#[allow(dead_code)]
impl Tileset {
    pub fn new(tile_width: u32, tile_height: u32) -> Self {
        Self { tile_width, tile_height }
    }
}
