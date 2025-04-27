use crate::config;
use ::rand::Rng;
use ::rand::rngs::ThreadRng;
use ::rand::thread_rng;
use macroquad::prelude::*;

// Represents a single particle
#[derive(Debug, Clone)]
pub struct Particle {
    pub position: Vec2,
    pub prev_position: Vec2,
    pub velocity: Vec2,
    pub color: Color,
    pub lifetime: f32, // Time remaining in seconds
    pub initial_lifetime: f32,
}

impl Particle {
    fn new(position: Vec2, velocity: Vec2, color: Color, lifetime: f32) -> Self {
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
        self.color.a = fade_factor;
    }

    // Check if particle has expired
    fn is_alive(&self) -> bool {
        self.lifetime > 0.0
    }
}

// Manages a collection of particles
#[derive(Debug)]
pub struct ParticleSystem {
    pub particles: Vec<Particle>,
    rng: ThreadRng, // Use ThreadRng directly
}

// Implementation for ParticleSystem
impl ParticleSystem {
    pub fn new() -> Self {
        ParticleSystem {
            particles: Vec::new(),
            rng: thread_rng(), // Use thread_rng() directly
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
        position: Vec2,
        base_color: Color,
        count: usize,
        max_speed: f32,
        lifetime: f32,
    ) {
        for _ in 0..count {
            // Use r#gen for raw identifier
            let angle = self.rng.r#gen::<f32>() * std::f32::consts::TAU;
            let speed = self.rng.r#gen::<f32>() * max_speed;
            let velocity = Vec2::new(angle.cos() * speed, angle.sin() * speed);
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
    pub fn spawn_muzzle_flash(&mut self, position: Vec2, direction_degrees: f64) {
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
            let velocity = Vec2::new(angle.cos() * speed, angle.sin() * speed);

            // Color (e.g., white/yellow)
            let mut color = YELLOW;
            color.a = 0.7;

            self.particles.push(Particle::new(
                position,
                velocity,
                color,
                lifetime * (0.8 + self.rng.r#gen::<f32>() * 0.4),
            ));
        }
    }

    /// Spawns particles along the path a projectile traveled in a tick.
    pub fn spawn_projectile_trail(
        &mut self,
        start_pos: Vec2,
        end_pos: Vec2,
        count: usize,
        lifetime: f32,
    ) {
        let direction = end_pos - start_pos;
        let distance = direction.length();

        // Don't spawn if it didn't move much
        let min_distance_threshold = config::UNIT_SIZE / 10.0;
        if distance < min_distance_threshold as f32 {
            return;
        }

        for i in 1..=count {
            let t = i as f32 / (count + 1) as f32; // Distribute along the path (exclude exact start/end)
            let position = start_pos.lerp(end_pos, t);

            // Give particles a slight random drift, perpendicular to the trail direction
            let perpendicular_dir = Vec2::new(-direction.y, direction.x).normalize_or_zero();
            let drift_speed = self.rng.r#gen_range(0.0..=(config::UNIT_SIZE * 0.5)) as f32; // Small drift
            let drift_velocity = perpendicular_dir * drift_speed * (self.rng.r#gen::<f32>() - 0.5) * 2.0; // Random direction

            // Base velocity can be zero or slightly backward to simulate dissipating smoke
            let base_velocity = -direction.normalize_or_zero() * self.rng.r#gen_range(0.0..=(config::UNIT_SIZE*0.1)) as f32;

            let final_velocity = base_velocity + drift_velocity;

            // Trail color (e.g., light gray, fading)
            let mut color = LIGHTGRAY;
            color.a = 0.6; // Start semi-transparent

            // Slightly randomized lifetime
            let particle_lifetime = lifetime * (0.8 + self.rng.r#gen::<f32>() * 0.4);

            self.particles.push(Particle::new(
                position,
                final_velocity,
                color,
                particle_lifetime,
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
}

// Test module for particles
#[cfg(test)]
mod tests {
    use super::*;
    use macroquad::color::{BLUE, RED};
    use macroquad::prelude::Vec2;

    #[test]
    fn test_particle_new_and_lifetime() {
        let p = Particle::new(Vec2::new(0.0, 0.0), Vec2::new(0.0, 0.0), RED, 1.0);
        assert!(p.is_alive());
        assert_eq!(p.lifetime, 1.0);
    }

    #[test]
    fn test_particle_update_lifetime() {
        let mut p = Particle::new(Vec2::new(0.0, 0.0), Vec2::new(0.0, 0.0), RED, 1.0);
        p.update(0.6);
        assert!(p.is_alive());
        assert!((p.lifetime - 0.4).abs() < f32::EPSILON);
        p.update(0.5); // Go past 0
        assert!(!p.is_alive());
        assert!(p.lifetime <= 0.0);
    }

    #[test]
    fn test_particle_update_position() {
        let mut p = Particle::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, -5.0), RED, 1.0);
        p.update(0.1);
        assert!((p.position.x - 1.0).abs() < f32::EPSILON);
        assert!((p.position.y - -0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_particle_update_fade() {
        let mut p = Particle::new(Vec2::new(0.0, 0.0), Vec2::new(0.0, 0.0), RED, 2.0);
        assert!((p.color.a - 1.0).abs() < f32::EPSILON);
        p.update(1.0); // Half lifetime
        assert!((p.color.a - 0.5).abs() < f32::EPSILON);
        p.update(0.5); // 3/4 lifetime
        assert!((p.color.a - 0.25).abs() < f32::EPSILON);
        p.update(1.0); // Past lifetime
        assert!((p.color.a - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_particle_system_new() {
        let ps = ParticleSystem::new();
        assert!(ps.particles.is_empty());
    }

    #[test]
    fn test_particle_system_spawn_explosion() {
        let mut ps = ParticleSystem::new();
        ps.spawn_explosion(Vec2::new(0.0, 0.0), BLUE, 10, 100.0, 1.0);
        assert_eq!(ps.particles.len(), 10);
        // Check a property of one particle (e.g., color)
        assert_eq!(ps.particles[0].color, BLUE);
    }

    #[test]
    fn test_particle_system_update() {
        let mut ps = ParticleSystem::new();
        // Spawn particles with short lifetime
        ps.spawn_explosion(Vec2::new(0.0, 0.0), BLUE, 5, 100.0, 0.1);
        assert_eq!(ps.particles.len(), 5);

        ps.update(0.05); // Update, but not enough to kill
        assert_eq!(ps.particles.len(), 5);
        // Check movement (at least one particle should have moved)
        assert!(ps.particles[0].position != Vec2::new(0.0, 0.0));

        ps.update(0.2); // Update enough to kill all particles (max lifetime is ~0.15)
        assert!(ps.particles.is_empty());
    }
}
