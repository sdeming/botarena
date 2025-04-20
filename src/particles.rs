use crate::config; // <-- Import config
use crate::utils;
use rand::Rng; // Import the Rng trait
use raylib::prelude::*; // <-- Import utils

// Represents a single particle
#[derive(Debug, Clone)]
struct Particle {
    position: Vector2,
    prev_position: Vector2,
    velocity: Vector2,
    color: Color,
    lifetime: f32, // Time remaining in seconds
    initial_lifetime: f32,
}

impl Particle {
    fn new(position: Vector2, velocity: Vector2, color: Color, lifetime: f32) -> Self {
        Particle {
            position,
            prev_position: position,
            velocity,
            color,
            lifetime,
            initial_lifetime: lifetime,
        }
    }

    // Update particle state over time
    fn update(&mut self, dt: f32) {
        self.position += self.velocity * dt;
        self.lifetime -= dt;

        // Fade out effect
        let fade_factor = (self.lifetime / self.initial_lifetime).max(0.0);
        self.color.a = (fade_factor * 255.0) as u8;
    }

    // Check if particle has expired
    fn is_alive(&self) -> bool {
        self.lifetime > 0.0
    }
}

// Manages a collection of particles
#[derive(Debug)]
pub struct ParticleSystem {
    particles: Vec<Particle>,
    rng: rand::rngs::ThreadRng,
}

// Implementation for ParticleSystem
impl ParticleSystem {
    pub fn new() -> Self {
        ParticleSystem {
            particles: Vec::new(),
            rng: rand::thread_rng(),
        }
    }

    /// Updates the previous state fields for all particles.
    /// Should be called BEFORE simulation updates for the cycle.
    pub fn update_prev_state(&mut self) {
        for p in self.particles.iter_mut() {
            p.prev_position = p.position;
        }
    }

    // Spawns a burst of particles
    pub fn spawn_explosion(
        &mut self,
        position: Vector2,
        base_color: Color,
        count: usize,
        max_speed: f32,
        lifetime: f32,
    ) {
        for _ in 0..count {
            // Use r#gen for raw identifier
            let angle = self.rng.r#gen::<f32>() * std::f32::consts::TAU;
            let speed = self.rng.r#gen::<f32>() * max_speed;
            let velocity = Vector2::new(angle.cos() * speed, angle.sin() * speed);
            let particle_lifetime = lifetime * (0.5 + self.rng.r#gen::<f32>() * 0.5);
            let particle_color = base_color;

            self.particles.push(Particle::new(
                position,
                velocity,
                particle_color,
                particle_lifetime,
            ));
        }
    }

    /// Spawns a short, directional burst of particles for muzzle flash.
    pub fn spawn_muzzle_flash(&mut self, position: Vector2, direction_degrees: f64) {
        let count = 5; // Small number of particles
        let lifetime = 0.15; // Very short life
        let base_speed = config::UNIT_SIZE as f32 * 8.0; // Moderate speed
        let spread_angle: f64 = 15.0; // Degrees <-- Specify type as f64

        let base_angle_rad = direction_degrees.to_radians() as f32;
        let spread_rad = spread_angle.to_radians() as f32;

        for _ in 0..count {
            // Angle within the spread cone
            let angle_offset = (self.rng.r#gen::<f32>() - 0.5) * spread_rad;
            let angle = base_angle_rad + angle_offset;

            // Speed variation
            let speed = base_speed * (0.7 + self.rng.r#gen::<f32>() * 0.6);
            let velocity = Vector2::new(angle.cos() * speed, angle.sin() * speed);

            // Color (e.g., white/yellow)
            let color = Color::YELLOW.alpha(0.7); // <-- Use alpha()

            self.particles.push(Particle::new(
                position,
                velocity,
                color,
                lifetime * (0.8 + self.rng.r#gen::<f32>() * 0.4),
            ));
        }
    }

    // Update all active particles based on fixed cycle duration
    pub fn update(&mut self, dt: f32) {
        self.particles.retain_mut(|p| {
            p.update(dt);
            p.is_alive()
        });
    }

    // Draw all active particles, interpolating positions
    pub fn draw(
        &self,
        d: &mut RaylibDrawHandle,
        screen_width: i32,
        screen_height: i32,
        alpha: f32,
    ) {
        for particle in &self.particles {
            // Interpolate position using utils
            let interp_pos_world = Vector2 {
                x: utils::lerp(particle.prev_position.x, particle.position.x, alpha),
                y: utils::lerp(particle.prev_position.y, particle.position.y, alpha),
            };

            // Convert interpolated world coordinates to screen coordinates
            let screen_pos = Vector2 {
                x: (interp_pos_world.x as f64 * screen_width as f64) as f32,
                y: (interp_pos_world.y as f64 * screen_height as f64) as f32,
            };

            d.draw_circle_v(screen_pos, 2.0, particle.color);
        }
    }
}

// Test module for particles
#[cfg(test)]
mod tests {
    use super::*; // Import items from outer module
    use raylib::prelude::Color;

    #[test]
    fn test_particle_new_and_lifetime() {
        let p = Particle::new(Vector2::zero(), Vector2::zero(), Color::RED, 1.0);
        assert!(p.is_alive());
        assert_eq!(p.lifetime, 1.0);
    }

    #[test]
    fn test_particle_update_lifetime() {
        let mut p = Particle::new(Vector2::zero(), Vector2::zero(), Color::RED, 1.0);
        p.update(0.6);
        assert!(p.is_alive());
        assert!((p.lifetime - 0.4).abs() < f32::EPSILON);
        p.update(0.5); // Go past 0
        assert!(!p.is_alive());
        assert!(p.lifetime <= 0.0);
    }

    #[test]
    fn test_particle_update_position() {
        let mut p = Particle::new(Vector2::zero(), Vector2::new(10.0, -5.0), Color::RED, 1.0);
        p.update(0.1);
        assert!((p.position.x - 1.0).abs() < f32::EPSILON);
        assert!((p.position.y - -0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_particle_update_fade() {
        let mut p = Particle::new(Vector2::zero(), Vector2::zero(), Color::RED, 2.0);
        assert_eq!(p.color.a, 255);
        p.update(1.0); // Half lifetime
        assert_eq!(p.color.a, 127);
        p.update(0.5); // 3/4 lifetime
        assert_eq!(p.color.a, 63);
        p.update(1.0); // Past lifetime
        assert_eq!(p.color.a, 0);
    }

    #[test]
    fn test_particle_system_new() {
        let ps = ParticleSystem::new();
        assert!(ps.particles.is_empty());
    }

    #[test]
    fn test_particle_system_spawn_explosion() {
        let mut ps = ParticleSystem::new();
        ps.spawn_explosion(Vector2::zero(), Color::BLUE, 10, 100.0, 1.0);
        assert_eq!(ps.particles.len(), 10);
        // Check a property of one particle (e.g., color)
        assert_eq!(ps.particles[0].color, Color::BLUE);
    }

    #[test]
    fn test_particle_system_update() {
        let mut ps = ParticleSystem::new();
        // Spawn particles with short lifetime
        ps.spawn_explosion(Vector2::zero(), Color::BLUE, 5, 100.0, 0.1);
        assert_eq!(ps.particles.len(), 5);

        ps.update(0.05); // Update, but not enough to kill
        assert_eq!(ps.particles.len(), 5);
        // Check movement (at least one particle should have moved)
        assert!(ps.particles[0].position != Vector2::zero());

        ps.update(0.2); // Update enough to kill all particles (max lifetime is ~0.15)
        assert!(ps.particles.is_empty());
    }
}
