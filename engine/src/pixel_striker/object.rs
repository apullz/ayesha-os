use image::RgbaImage;

/// Generic game object sprite (terminal, drone, etc.) for future use.
#[derive(Clone, Debug)]
pub struct GameObject {
    pub width: u32,
    pub height: u32,
    pub data: Vec<[u8; 3]>,
}

impl GameObject {
    pub fn draw(&self, _img: &mut RgbaImage, _ox: u32, _oy: u32, _pixel_size: u32) {
        // stub for future object drawing
    }
}
