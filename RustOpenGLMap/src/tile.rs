const MAX_ZOOM: u8 = 19;
use image::RgbaImage;
use std::hash::{Hash, Hasher};
// Added for managing loading state, optional

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TileState {
    Loading {
        texture_id: gl::types::GLuint,
        source_tile: TilePos,
        target_tile: TilePos,
    }, // source tile not loaded, showing highest possible tile
    Loaded {
        texture_id: gl::types::GLuint,
        source_tile: TilePos,
    }, // source_tile is the tile the texture actually represents
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TileLoad {
    Loading {
        texture: RgbaImage,
        source_tile: TilePos,
        target_tile: TilePos,
    }, // source tile not loaded, showing highest possible tile
    Loaded {
        texture: RgbaImage,
        source_tile: TilePos,
    }, // source_tile is the tile the texture actually represents
    Failed,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct TilePos {
    pub z: u8,
    pub x: u32,
    pub y: u32,
    pub m: u8,
}
impl Hash for TilePos {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // order matters; keep it consistent with Eq/PartialEq
        self.z.hash(state);
        self.x.hash(state);
        self.y.hash(state);
        self.m.hash(state);
    }
}

impl TilePos {
    pub fn new() -> Self {
        Self {
            z: 0,
            x: 0,
            y: 0,
            m: 0,
        }
    }

    /// Return the child tile that lies under (u,v) in [0,1]².
    /// SDL’s Y axis grows downward, OSM’s Y grows *down*, too,
    /// so no extra flip is needed.
    pub fn zoom_in(&mut self, u: f64, v: f64) {
        if self.z >= MAX_ZOOM {
            return;
        } // OSM max
        self.x = self.x * 2 + if u >= 0.5 { 1 } else { 0 };
        self.y = self.y * 2 + if v >= 0.5 { 1 } else { 0 };
        self.z += 1;
    }

    pub fn zoom_in_tile(&mut self, other: TilePos) -> (i32, i32) {
        if self.z >= MAX_ZOOM {
            return (0, 0);
        } // OSM max
        self.x = self.x * 2;
        self.y = self.y * 2;
        self.z += 1;
        let x: i32;
        let y: i32;
        if self.x < other.x {
            x = 1;
            self.x += 1;
        } else {
            x = 0;
        }
        if self.y < other.y {
            y = 1;
            self.y += 1;
        } else {
            y = 0;
        }
        (x, y)
    }

    /// Return the parent tile that lies above (u,v) in [0,1]².
    /// SDL’s Y axis grows downward, OSM’s Y grows *down*, too,
    /// so no extra flip is needed.
    pub fn zoom_out(&mut self) {
        if self.z <= 0 {
            return;
        } // OSM max
        self.x = self.x / 2;
        self.y = self.y / 2;
        self.z -= 1;
    }

    pub fn get_crop(&mut self, child: &TilePos) -> (i32, i32, i32, i32) {
        let dz = (child.z as i32 - self.z as i32).clamp(0, 8);
        let p = 1 << dz;
        let s = 256 / p;
        let xx = (child.x as i32 % p) * s;
        let yy = (child.y as i32 % p) * s;
        (xx, yy, s, s)

        // let mut zoomed_tile = self;
        // let mut xx:i32 = 0;
        // let mut yy:i32 = 0;
        // for i in 1..z_diff+1
        // {
        //     let (x, y) = zoomed_tile.zoom_in_tile(other);
        //     let zoom = (256 / (1 << i));
        //     xx += x * zoom;
        //     yy += y * zoom;
        // }
    }
}
