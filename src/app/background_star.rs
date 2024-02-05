use super::*;

pub struct BackgroundStar {
    pub pos: Vec2,
    pub radius: f32,
    pub parallax: f32,
}

pub fn generate(count: usize) -> Vec<BackgroundStar> {
    (0..count)
        .into_iter()
        .map(|_| {
            let pos = vec2(
                rand::random::<f32>() * 2. - 1.,
                rand::random::<f32>() * 2. - 1.,
            );
            let parallax = (rand::random::<f32>() * 0.02 + 0.00005).powi(2);
            let size = rand::random::<f32>() * 0.004 + 0.001;

            BackgroundStar {
                pos,
                radius: size,
                parallax,
            }
        })
        .collect()
}
