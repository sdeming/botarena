use crate::audio::AudioManager;
use crate::config;
use crate::config::*;
use crate::particles::ParticleSystem;
use crate::robot::{Robot, RobotStatus};
use crate::types::*;
use ::rand::prelude::*;
use macroquad::prelude::*;
use macroquad::prelude::{ORANGE, SKYBLUE, Vec2, YELLOW};

// Represents an obstacle in the arena
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Obstacle {
    pub position: Point, // Center position in coordinate units
}

// Represents the game arena
#[derive(Debug)]
pub struct Arena {
    pub width: f64,       // Width in coordinate units (typically 1.0)
    pub height: f64,      // Height in coordinate units (typically 1.0)
    pub grid_width: u32,  // Width in grid units
    pub grid_height: u32, // Height in grid units
    pub unit_size: f64,   // Size of one grid unit in coordinate units
    pub obstacles: Vec<Obstacle>,
    pub projectiles: Vec<Projectile>,
}

impl Arena {
    pub fn new() -> Self {
        let width = ARENA_WIDTH_UNITS as f64 * UNIT_SIZE;
        let height = ARENA_HEIGHT_UNITS as f64 * UNIT_SIZE;

        Arena {
            width,
            height,
            grid_width: ARENA_WIDTH_UNITS,
            grid_height: ARENA_HEIGHT_UNITS,
            unit_size: UNIT_SIZE,
            obstacles: Vec::new(),
            projectiles: Vec::new(),
        }
    }

    // Places obstacles randomly based on configured density
    pub fn place_obstacles(&mut self) {
        let mut rng = thread_rng();
        let total_cells = self.grid_width * self.grid_height;
        let num_obstacles = (total_cells as f32 * OBSTACLE_DENSITY).floor() as u32;

        log::info!("Placing {} obstacles...", num_obstacles);
        self.obstacles.clear(); // Clear existing obstacles

        // Keep track of occupied grid cells to avoid duplicates
        let mut occupied_cells = std::collections::HashSet::new();

        for _ in 0..num_obstacles {
            // Find an empty cell
            loop {
                let grid_x = rng.gen_range(0..self.grid_width);
                let grid_y = rng.gen_range(0..self.grid_height);

                // TODO: Add logic to avoid placing obstacles near potential starting positions

                if occupied_cells.insert((grid_x, grid_y)) {
                    let position = self.grid_to_world(grid_x, grid_y);
                    self.obstacles.push(Obstacle { position });
                    break; // Found an empty cell, move to next obstacle
                }
                // If cell is already occupied, loop again
            }
        }
        log::info!("Obstacles placed.");
    }

    // Checks if a given point collides with any obstacle's bounding box
    // Note: This checks the point itself, not a robot's bounding box yet.
    pub fn check_collision(&self, point: Point) -> bool {
        let half_unit = self.unit_size / 2.0;
        for obstacle in &self.obstacles {
            let obs_x = obstacle.position.x;
            let obs_y = obstacle.position.y;

            // Simple AABB check (Axis-Aligned Bounding Box)
            if point.x >= obs_x - half_unit
                && point.x < obs_x + half_unit
                && point.y >= obs_y - half_unit
                && point.y < obs_y + half_unit
            {
                return true; // Collision detected
            }
        }
        false // No collision detected
    }

    // Converts grid coordinates (u32) to world coordinates (f64)
    // Returns the center of the grid cell
    pub fn grid_to_world(&self, grid_x: u32, grid_y: u32) -> Point {
        Point {
            x: (grid_x as f64 + 0.5) * self.unit_size,
            y: (grid_y as f64 + 0.5) * self.unit_size,
        }
    }
    
    // Adds a projectile to the arena's list
    pub fn spawn_projectile(&mut self, projectile: Projectile) {
        log::debug!(
            "Spawning projectile from robot {} at ({:.2}, {:.2}) dir {:.2}",
            projectile.source_robot,
            projectile.position.x,
            projectile.position.y,
            projectile.direction
        );
        self.projectiles.push(projectile);
    }

    // Updates all active projectiles in the arena using sub-stepping for collision detection
    pub fn update_projectiles(
        &mut self,
        robots: &mut [Robot],
        particle_system: &mut ParticleSystem,
        audio_manager: &AudioManager,
    ) {
        let mut i = 0;
        let sub_steps = config::PROJECTILE_SUB_STEPS;

        while i < self.projectiles.len() {
            let mut projectile_removed = false;
            let projectile = self.projectiles[i]; // Copy for immutable data access

            // Calculate total movement for the cycle
            let angle_rad = projectile.direction.to_radians();
            let total_dx = angle_rad.cos() * projectile.speed * self.unit_size;
            let total_dy = angle_rad.sin() * projectile.speed * self.unit_size;

            // Calculate movement per sub-step
            let step_dx = total_dx / sub_steps as f64;
            let step_dy = total_dy / sub_steps as f64;

            // Update previous position only once at the beginning of the cycle
            self.projectiles[i].prev_position = self.projectiles[i].position;

            // --- Sub-step Loop ---
            for step in 0..sub_steps {
                // Move projectile by one sub-step
                self.projectiles[i].position.x += step_dx;
                self.projectiles[i].position.y += step_dy;

                let current_pos = self.projectiles[i].position;
                let source_id = projectile.source_robot;
                let proj_power = projectile.power;
                let proj_base_damage = projectile.base_damage;

                // Check for collisions with arena boundaries
                if current_pos.x < 0.0
                    || current_pos.x > self.width
                    || current_pos.y < 0.0
                    || current_pos.y > self.height
                {
                    log::debug!(
                        "Projectile hit boundary at ({:.2}, {:.2}) on sub-step {}",
                        current_pos.x,
                        current_pos.y,
                        step + 1
                    );
                    let hit_position = Vec2::new(current_pos.x as f32, current_pos.y as f32);
                    particle_system.spawn_explosion(
                        hit_position,
                        SKYBLUE,
                        60,
                        config::UNIT_SIZE as f32 * 5.0,
                        0.6,
                    );
                    self.projectiles.swap_remove(i);
                    projectile_removed = true;
                    break; // Exit sub-step loop
                }

                // Check for collisions with obstacles
                if self.check_collision(current_pos) {
                    log::debug!(
                        "Projectile hit obstacle at ({:.2}, {:.2}) on sub-step {}",
                        current_pos.x,
                        current_pos.y,
                        step + 1
                    );
                    let hit_position = Vec2::new(current_pos.x as f32, current_pos.y as f32);
                    particle_system.spawn_explosion(
                        hit_position,
                        YELLOW,
                        50,
                        config::UNIT_SIZE as f32 * 4.0,
                        0.5,
                    );
                    self.projectiles.swap_remove(i);
                    projectile_removed = true;
                    break; // Exit sub-step loop
                }

                // Check for collisions with robots
                for robot in robots.iter_mut() {
                    if robot.id == source_id || robot.status == RobotStatus::Destroyed {
                        continue;
                    }
                    let dist_sq = (robot.position.x - current_pos.x).powi(2)
                        + (robot.position.y - current_pos.y).powi(2);
                    let collision_radius_sq = (self.unit_size / 2.0).powi(2);

                    if dist_sq < collision_radius_sq {
                        log::debug!(
                            "Projectile hit robot {} at ({:.2}, {:.2}) on sub-step {}",
                            robot.id,
                            current_pos.x,
                            current_pos.y,
                            step + 1
                        );
                        let hit_position = Vec2::new(current_pos.x as f32, current_pos.y as f32);
                        let particle_count = (proj_power * 75.0 + 20.0) as usize;
                        let particle_lifetime = 0.6 + proj_power * 0.6;
                        particle_system.spawn_explosion(
                            hit_position,
                            ORANGE,
                            particle_count,
                            config::UNIT_SIZE as f32 * 6.0,
                            particle_lifetime as f32,
                        );

                        let damage = proj_base_damage * proj_power;
                        robot.health -= damage;
                        audio_manager.play_hit();
                        log::info!(
                            "Robot {} took {:.2} damage, health remaining: {:.2}",
                            robot.id,
                            damage,
                            robot.health
                        );
                        if robot.health <= 0.0 {
                            robot.health = 0.0;
                            robot.status = RobotStatus::Destroyed;
                            audio_manager.play_death();
                            log::info!("Robot {} destroyed!", robot.id);
                        }
                        self.projectiles.swap_remove(i);
                        projectile_removed = true;
                        break; // Exit robot loop
                    }
                }
                if projectile_removed {
                    break;
                } // Exit sub-step loop if robot was hit
            } // End of sub-step loop

            // Only increment `i` if the projectile wasn't removed during sub-steps
            if !projectile_removed {
                i += 1;
            }
            // If removed, the swap_remove already handled the next element, so don't increment i
        }
    }

    /// Calculates the distance from a robot's center point to the point where its edge
    /// would first collide with a wall or obstacle along a given angle.
    pub fn distance_to_collision(&self, start_point: Point, angle_degrees: f64) -> f64 {
        let angle_rad = angle_degrees.to_radians();
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();
        let robot_radius = self.unit_size / 2.0;

        let _min_dist = f64::INFINITY;

        // --- Check Walls ---
        // Calculate distance for the CENTER point hitting the wall first.
        let mut min_dist_wall_center = f64::INFINITY;
        if cos_a.abs() > 1e-9 {
            let dist_x0 = -start_point.x / cos_a;
            if dist_x0 > 1e-9 {
                let y_at_x0 = start_point.y + dist_x0 * sin_a;
                if y_at_x0 >= 0.0 && y_at_x0 <= self.height {
                    min_dist_wall_center = min_dist_wall_center.min(dist_x0);
                }
            }
            let dist_xw = (self.width - start_point.x) / cos_a;
            if dist_xw > 1e-9 {
                let y_at_xw = start_point.y + dist_xw * sin_a;
                if y_at_xw >= 0.0 && y_at_xw <= self.height {
                    min_dist_wall_center = min_dist_wall_center.min(dist_xw);
                }
            }
        }
        if sin_a.abs() > 1e-9 {
            let dist_y0 = -start_point.y / sin_a;
            if dist_y0 > 1e-9 {
                let x_at_y0 = start_point.x + dist_y0 * cos_a;
                if x_at_y0 >= 0.0 && x_at_y0 <= self.width {
                    min_dist_wall_center = min_dist_wall_center.min(dist_y0);
                }
            }
            let dist_yh = (self.height - start_point.y) / sin_a;
            if dist_yh > 1e-9 {
                let x_at_yh = start_point.x + dist_yh * cos_a;
                if x_at_yh >= 0.0 && x_at_yh <= self.width {
                    min_dist_wall_center = min_dist_wall_center.min(dist_yh);
                }
            }
        }

        // Adjust wall distance for robot radius. Clamp at 0.0.
        let mut min_dist_wall_edge = (min_dist_wall_center - robot_radius).max(0.0);

        // --- Check Obstacles (Swept Circle-AABB Intersection) ---
        // This already calculates distance for the edge hitting the obstacle.
        let obstacle_half_unit = self.unit_size / 2.0;
        let inv_dx = if cos_a.abs() > 1e-9 {
            1.0 / cos_a
        } else {
            f64::INFINITY
        };
        let inv_dy = if sin_a.abs() > 1e-9 {
            1.0 / sin_a
        } else {
            f64::INFINITY
        };

        for obstacle in &self.obstacles {
            let obs_pos = obstacle.position;

            // Calculate the bounds of the AABB expanded by the robot's radius
            let expanded_min_x = obs_pos.x - obstacle_half_unit - robot_radius;
            let expanded_max_x = obs_pos.x + obstacle_half_unit + robot_radius;
            let expanded_min_y = obs_pos.y - obstacle_half_unit - robot_radius;
            let expanded_max_y = obs_pos.y + obstacle_half_unit + robot_radius;

            // Perform Ray-AABB intersection test against the *expanded* AABB
            let t1x = (expanded_min_x - start_point.x) * inv_dx;
            let t2x = (expanded_max_x - start_point.x) * inv_dx;
            let t1y = (expanded_min_y - start_point.y) * inv_dy;
            let t2y = (expanded_max_y - start_point.y) * inv_dy;

            let tmin = t1x.min(t2x).max(t1y.min(t2y));
            let tmax = t1x.max(t2x).min(t1y.max(t2y));

            if tmax >= 0.0 && tmin <= tmax {
                // Check for valid intersection interval
                if tmin > 1e-9 {
                    // Intersection is ahead
                    min_dist_wall_edge = min_dist_wall_edge.min(tmin);
                } else {
                    // Robot starts overlapping or exactly touching expanded box
                    min_dist_wall_edge = 0.0;
                    break; // Minimum possible distance found
                }
            }
        }

        min_dist_wall_edge
    }

    /// First pass of the AOI (area of interest) detector
    /// Takes a slice of mutable robots to update their AOI fields
    pub fn update_all_robots_aoi(&mut self, robots: &mut [Robot]) {
        // Clear existing AOIs
        for robot in robots.iter_mut() {
            robot.aoi.clear();
        }

        // Calculate new AOIs - each robot's AOI contains IDs of robots in its scan range
        for i in 0..robots.len() {
            let robot_position = robots[i].position;
            let _robot_id = robots[i].id;

            for j in 0..robots.len() {
                if i == j {
                    continue; // Skip self
                }

                let other_robot = &robots[j];
                if other_robot.status == RobotStatus::Destroyed {
                    continue; // Skip destroyed robots
                }

                let distance = robot_position.distance(&other_robot.position);

                // Only add robots to AOI that are within the scan distance
                if distance <= config::SCAN_DISTANCE {
                    robots[i].aoi.push(other_robot.id);
                }
            }
        }
    }

    /// Adds an obstacle at the given robot's position (for wreckage)
    pub fn add_obstacle_at_robot(&mut self, robot: &Robot) {
        self.obstacles.push(Obstacle {
            position: robot.position,
        });
    }
}

// Default implementation for Arena
impl Default for Arena {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;
    use crate::particles::ParticleSystem;
    use crate::robot::Robot;
    use crate::types::{Point, Projectile};

    #[test]
    fn test_projectile_movement() {
        let mut arena = Arena::new();
        let start_pos = Point { x: 0.5, y: 0.5 };
        let projectile = Projectile {
            position: start_pos,
            prev_position: start_pos,
            direction: 0.0, // Moving right
            speed: 1.0,     // 1 unit per cycle
            power: 1.0,
            base_damage: 10.0,
            source_robot: 0,
        };
        arena.spawn_projectile(projectile);

        let mut robots = vec![];
        let mut particle_system = ParticleSystem::new();
        let audio_manager = AudioManager::new();
        arena.update_projectiles(&mut robots, &mut particle_system, &audio_manager);

        assert_eq!(arena.projectiles.len(), 1);
        let updated_proj = &arena.projectiles[0];
        let expected_x = start_pos.x + 1.0 * config::UNIT_SIZE; // Moved 1 unit (speed * unit_size)
        assert!((updated_proj.position.x - expected_x).abs() < 1e-9);
        assert!((updated_proj.position.y - start_pos.y).abs() < 1e-9); // Y should not change
    }

    #[test]
    fn test_projectile_boundary_collision() {
        let mut arena = Arena::new();
        // Spawn projectile close to the right edge (Arena width is 1.0 by default)
        let start_pos = Point { x: 0.98, y: 0.5 };
        let projectile = Projectile {
            position: start_pos,
            prev_position: start_pos,
            direction: 0.0, // Moving right
            speed: 1.0,
            power: 1.0,
            base_damage: 10.0,
            source_robot: 0,
        };
        arena.spawn_projectile(projectile);

        let mut robots = vec![];
        let mut particle_system = ParticleSystem::new();
        let audio_manager = AudioManager::new();
        arena.update_projectiles(&mut robots, &mut particle_system, &audio_manager);

        assert!(
            arena.projectiles.is_empty(),
            "Projectile should be removed after hitting boundary"
        );
    }

    #[test]
    fn test_projectile_obstacle_collision() {
        let mut arena = Arena::new();
        // Place an obstacle
        let obstacle_pos = arena.grid_to_world(10, 10); // Middle obstacle
        arena.obstacles.push(Obstacle {
            position: obstacle_pos,
        });

        // Spawn projectile just left of the obstacle, moving right
        let start_pos = Point {
            x: obstacle_pos.x - config::UNIT_SIZE * 0.6,
            y: obstacle_pos.y,
        };
        let projectile = Projectile {
            position: start_pos,
            prev_position: start_pos,
            direction: 0.0, // Moving right
            speed: 1.0,
            power: 1.0,
            base_damage: 10.0,
            source_robot: 0,
        };
        arena.spawn_projectile(projectile);

        let mut robots = vec![];
        let mut particle_system = ParticleSystem::new();
        let audio_manager = AudioManager::new();
        arena.update_projectiles(&mut robots, &mut particle_system, &audio_manager);

        assert!(
            arena.projectiles.is_empty(),
            "Projectile should be removed after hitting obstacle"
        );
    }

    #[test]
    fn test_projectile_robot_collision_and_damage() {
        let mut arena = Arena::new();
        let robot1_start = Point { x: 0.25, y: 0.5 };
        let robot2_start = Point { x: 0.75, y: 0.5 };
        let arena_center = Point { x: 0.5, y: 0.5 }; // Define center point
        let mut robot1 = Robot::new(1, "TestRobot1".to_string(), robot1_start, arena_center);
        robot1.status = RobotStatus::Active; // Manually set active for test
        let mut robot2 = Robot::new(2, "TestRobot2".to_string(), robot2_start, arena_center);
        robot2.status = RobotStatus::Active; // <-- Manually set status for test
        let mut particle_system = ParticleSystem::new(); // <-- Create dummy particle system
        let audio_manager = AudioManager::new(); // <-- Create dummy manager

        // Spawn projectile from robot 1 aimed at robot 2
        let proj_start_pos = Point {
            x: robot1_start.x + config::UNIT_SIZE,
            y: robot1_start.y,
        };
        let projectile = Projectile {
            position: proj_start_pos,
            prev_position: proj_start_pos,
            direction: 0.0,    // Moving right
            speed: 9.0,        // Adjusted speed to land exactly on target center after 1 cycle
            power: 0.5,        // Power affects damage
            base_damage: 20.0, // Base damage
            source_robot: 1,   // Fired by robot 1
        };
        arena.spawn_projectile(projectile);

        let initial_health_r2 = robot2.health;
        let mut robots = vec![robot1, robot2]; // Pass robots as mutable slice

        arena.update_projectiles(&mut robots, &mut particle_system, &audio_manager);

        assert!(
            arena.projectiles.is_empty(),
            "Projectile should be removed after hitting robot"
        );
        // Check status (this assertion should now pass)
        assert_eq!(
            robots[1].status,
            crate::robot::RobotStatus::Active,
            "Robot 2 should not be destroyed yet"
        );

        let expected_damage = 20.0 * 0.5; // base_damage * power
        assert!(
            (robots[1].health - (initial_health_r2 - expected_damage)).abs() < 1e-9,
            "Robot 2 did not take correct damage"
        );
        assert_eq!(
            robots[0].health, 100.0,
            "Robot 1 health should be unchanged"
        ); // Verify R1 health

        // Test lethal hit
        robots[1].health = 5.0; // Low health
        robots[1].status = RobotStatus::Active; // Ensure status is Active for lethal test too
        let mut particle_system_lethal = ParticleSystem::new(); // Separate system for lethal test
        let audio_manager_lethal = AudioManager::new(); // <-- Create dummy manager for lethal
        let proj2_start_pos = Point {
            x: robots[0].position.x + config::UNIT_SIZE,
            y: robots[0].position.y,
        };
        let projectile2 = Projectile {
            position: proj2_start_pos,
            prev_position: proj2_start_pos,
            direction: 0.0, // Moving right
            speed: 9.0,     // Adjusted speed
            power: 0.5,
            base_damage: 20.0,
            source_robot: 1,
        };
        arena.spawn_projectile(projectile2);
        arena.update_projectiles(
            &mut robots,
            &mut particle_system_lethal,
            &audio_manager_lethal,
        );
        assert!(
            arena.projectiles.is_empty(),
            "Lethal projectile should be removed"
        );
        assert_eq!(
            robots[1].health, 0.0,
            "Robot 2 health should be 0 after lethal hit"
        );
        assert_eq!(
            robots[1].status,
            crate::robot::RobotStatus::Destroyed,
            "Robot 2 should be destroyed"
        );
    }

    #[test]
    fn test_projectile_ignores_source_robot() {
        let mut arena = Arena::new();
        let robot1_start = Point { x: 0.5, y: 0.5 };
        let arena_center = Point { x: 0.5, y: 0.5 }; // Define center point
        let mut robot1 = Robot::new(1, "TestRobot1".to_string(), robot1_start, arena_center);
        robot1.status = RobotStatus::Active; // Set active
        let mut particle_system = ParticleSystem::new(); // <-- Create dummy particle system
        let audio_manager = AudioManager::new(); // <-- Create dummy manager

        // Spawn projectile from robot 1 aimed back at itself (180 deg)
        // It starts 1 unit away, but will pass through the origin point on next cycle
        let proj_start_pos = Point {
            x: robot1_start.x - config::UNIT_SIZE,
            y: robot1_start.y,
        };
        let projectile = Projectile {
            position: proj_start_pos,
            prev_position: proj_start_pos,
            direction: 180.0, // Moving left
            speed: 1.0,       // 1 unit per cycle
            power: 1.0,
            base_damage: 100.0,
            source_robot: 1, // Fired by robot 1
        };
        arena.spawn_projectile(projectile);

        let initial_health_r1 = robot1.health;
        let mut robots = vec![robot1];

        // Cycle 1: Projectile moves left, passing through (0.5, 0.5)
        arena.update_projectiles(&mut robots, &mut particle_system, &audio_manager);

        assert_eq!(
            arena.projectiles.len(),
            1,
            "Projectile should not have been removed"
        );
        assert_eq!(
            robots[0].health, initial_health_r1,
            "Source robot health should be unchanged"
        );
    }
}
