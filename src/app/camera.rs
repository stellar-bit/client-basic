use super::*;

#[derive(Default)]
pub struct Camera {
    pub center: Vec2,
    pub offset: Vec2,
    pub zoom: f32,
    pub win_ratio: f32
}

impl Camera {
    pub fn new(zoom: f32, win_ratio: f32) -> Self {
        Camera {
            center: vec2(0., 0.),
            offset: vec2(0., 0.),
            zoom,
            win_ratio
        }
    }
    pub fn mp(&self) -> Vec2 {
        let mpy = 2f32.powf(self.zoom);
        let mpx = self.win_ratio*mpy;
        vec2(mpx, mpy)
    }
    pub fn to_real(&self, pos: Vec2) -> Vec2 {
        pos / self.mp() + self.center + self.offset
    }
    pub fn to_screen(&self, pos: Vec2) -> Vec2 {
        (pos - self.center - self.offset) * self.mp()
    }
    pub fn position(&self) -> Vec2 {
        self.center + self.offset
    }
}
