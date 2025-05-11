#[derive(Debug)]
pub struct Viewport {
    pub z: u8,
    pub center_x: f64,
    pub center_y: f64,
    pub rm_x: f64,
    pub rm_y: f64,
}

impl Viewport {
    pub fn pan(&mut self, dx: f64, dy: f64) {
        self.center_x += (dx);
        self.center_y += (dy);
        self.rm_x = 0.0;
        self.rm_y = 0.0;
    }

    pub fn zoom_in(&mut self) {
        if self.z < 19 {
            self.center_x *= 2.0;
            self.center_y *= 2.0;
            self.z += 1;
            self.pan(0.5, 0.5);
        }
    }

    pub fn zoom_out(&mut self) {
        if self.z > 0 {
            self.center_x /= 2.0;
            self.center_y /= 2.0;
            self.z -= 1;
            self.pan(-0.5, -0.5);
        }
    }

    pub fn center_on_pixel(&mut self, win_w: u32, win_h: u32, px: i32, py: i32) {
        // 1 tile  = 256 px   → 1 px = 1/256 tile
        let dx_win = ((px as f64 - (win_w as f64) / 2.0) / win_w as f64);
        let dy_win = ((py as f64 - (win_h as f64) / 2.0) / win_h as f64);

        let dx_tiles = dx_win * win_w as f64 / 256.0;
        let dy_tiles = dy_win * win_h as f64 / 256.0;

        self.pan(dx_tiles, dy_tiles);
    }

    /// Same as `center_on_pixel`, then zoom-in so that the clicked point
    /// stays under the cursor.
    pub fn zoom_in_at_pixel(&mut self, win_w: u32, win_h: u32, px: i32, py: i32) {
        self.center_on_pixel(win_w, win_h, px, py);
        self.zoom_in()
    }
}
