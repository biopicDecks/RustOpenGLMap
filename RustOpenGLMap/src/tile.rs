const MAX_ZOOM: u8 = 19;
use std::hash::{Hash, Hasher};

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
}
