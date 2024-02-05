use super::*;

pub trait Particle {
    fn update(&mut self, dt: f32);
    fn draw(&self) -> Vec<Shape<Txts>>;
    fn lifetime(&self) -> f32;
}

pub struct ShrinkingCircle {
    position: Vec2,
    radius: f32,
    lifetime: f32,
    velocity: Vec2,
    shrink_speed: f32,
    color: Color,
    end_color: Color,
}

impl ShrinkingCircle {
    pub fn new(
        position: Vec2,
        velocity: Vec2,
        radius: f32,
        lifetime: f32,
        start_color: Color,
        end_color: Color,
    ) -> Self {
        Self {
            position,
            radius,
            lifetime,
            velocity,
            shrink_speed: radius / lifetime * 0.9,
            color: start_color,
            end_color,
        }
    }
}

impl Particle for ShrinkingCircle {
    fn update(&mut self, dt: f32) {
        self.lifetime -= dt;
        self.radius -= self.shrink_speed * dt;
        self.position += self.velocity * dt;
        self.color = Color::from_rgb(
            self.color.r + (self.end_color.r - self.color.r) * dt / self.lifetime,
            self.color.g + (self.end_color.g - self.color.g) * dt / self.lifetime,
            self.color.b + (self.end_color.b - self.color.b) * dt / self.lifetime,
        );
    }
    fn draw(&self) -> Vec<Shape<Txts>> {
        if self.radius < 0. {
            return vec![];
        }
        let gtransform = GTransform::from_translation(self.position).inflate(self.radius);
        return vec![Shape::from_circle(20)
            .apply(gtransform)
            .set_color(self.color)];
    }
    fn lifetime(&self) -> f32 {
        self.lifetime
    }
}

pub struct ParticleSystem {
    particles: Vec<Box<dyn Particle>>,
}

impl ParticleSystem {
    pub fn new() -> Self {
        Self { particles: vec![] }
    }
    pub fn add_particle(&mut self, particle: Box<dyn Particle>) {
        self.particles.push(particle);
    }
    pub fn update(&mut self, dt: f32) {
        self.particles.retain(|particle| particle.lifetime() > 0.);
        for particle in &mut self.particles {
            particle.update(dt);
        }
    }
    pub fn draw(&self) -> Vec<Shape<Txts>> {
        self.particles
            .iter()
            .flat_map(|particle| particle.draw())
            .collect()
    }
}
