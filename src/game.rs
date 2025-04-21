use crate::arena::Arena;
use crate::config;
use crate::particles::ParticleSystem;
use crate::render::Renderer;
use crate::robot::{Robot, RobotStatus};
use crate::types::{ArenaCommand, Point};
use log::{error, info};
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::process;
use macroquad::prelude::{get_frame_time, next_frame, Vec2};

/// The Game struct encapsulates the state and logic for running the bot arena simulation
pub struct Game {
    pub arena: Arena,
    pub robots: Vec<Robot>,
    pub particle_system: ParticleSystem,
    pub current_turn: u32,
    pub current_cycle: u32,
    pub max_turns: u32,
    time_accumulator: f32,
    cycle_duration: f32,
    game_over: bool,
    winner: Option<u32>,
}

impl Game {
    /// Create a new game instance with the provided robot files
    pub fn new(robot_files: &[String], max_turns: u32) -> Result<Self, Box<dyn std::error::Error>> {
        // Create arena
        let arena = Arena::new();
        info!(
            "Arena created with {}x{} grid.",
            arena.grid_width, arena.grid_height
        );

        // Create predefined constants for robot programs
        let mut predefined_constants = HashMap::new();
        predefined_constants.insert("ARENA_WIDTH".to_string(), arena.grid_width as f64);
        predefined_constants.insert("ARENA_HEIGHT".to_string(), arena.grid_height as f64);

        // Check robot count
        let num_robots = robot_files.len();
        if num_robots > 4 {
            error!("Error: Maximum of 4 robots allowed.");
            process::exit(1);
        }

        // Load robots
        let mut robots = Vec::with_capacity(num_robots);
        info!("Simulating for a maximum of {} turns.", max_turns);

        // Define starting positions
        let offset = 2.0 * config::UNIT_SIZE;
        let positions = [
            Point {
                x: offset,
                y: offset,
            }, // Top-left
            Point {
                x: 1.0 - offset,
                y: offset,
            }, // Top-right
            Point {
                x: 1.0 - offset,
                y: 1.0 - offset,
            }, // Bottom-right
            Point {
                x: offset,
                y: 1.0 - offset,
            }, // Bottom-left
        ];

        // Load robot programs
        for (i, filename) in robot_files.iter().enumerate() {
            let robot_id = (i + 1) as u32;
            let position = positions[i];

            info!(
                "Loading and parsing program for Robot {} from file: {}",
                robot_id, filename
            );
            let program_content = match fs::read_to_string(filename) {
                Ok(content) => content,
                Err(e) => {
                    error!("Error reading file {}: {}", filename, e);
                    process::exit(1);
                }
            };

            // Parse the program using the predefined constants
            match crate::vm::parser::parse_assembly(&program_content, Some(&predefined_constants)) {
                Ok(parsed_program) => {
                    let mut robot = Robot::new(robot_id, position);
                    robot.load_program(parsed_program);
                    robots.push(robot);
                }
                Err(e) => {
                    error!(
                        "Error parsing program for Robot {} (file: {}): Line {}, {}",
                        robot_id, filename, e.line, e.message
                    );
                    process::exit(1);
                }
            }
        }
        info!("Loaded {} robots.", robots.len());

        // Initialize particle system
        let particle_system = ParticleSystem::new();
        info!("Particle system initialized.");

        Ok(Game {
            arena,
            robots,
            particle_system,
            current_turn: 1,
            current_cycle: 0,
            max_turns,
            time_accumulator: 0.0,
            cycle_duration: 1.0 / config::CYCLES_PER_TURN as f32,
            game_over: false,
            winner: None,
        })
    }

    /// Run the main game loop using the provided renderer
    pub async fn run(&mut self, renderer: &mut Renderer) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting main loop...");

        let mut announcement: Option<String> = None;
        let mut game_ended = false;

        while !Renderer::window_should_close() && self.current_turn <= self.max_turns && !self.game_over {
            // Time accumulation
            let frame_time = get_frame_time();
            self.time_accumulator += frame_time;

            // Fixed simulation update loop
            while self.time_accumulator >= self.cycle_duration {
                // Consume time for this cycle
                self.time_accumulator -= self.cycle_duration;

                self.update_simulation();

                // Break if max turns reached during this frame's updates
                if self.current_turn > self.max_turns {
                    break;
                }
            }

            // Draw frame
            renderer.draw_frame(
                &self.arena,
                &self.robots,
                &self.particle_system,
                self.current_turn,
                self.max_turns,
                self.current_cycle,
                config::CYCLES_PER_TURN,
                self.time_accumulator,
                self.cycle_duration,
                None,
            );
            next_frame().await;
        }

        // Prepare announcement message
        if self.game_over {
            game_ended = true;
            announcement = Some(
                if let Some(winner_id) = self.winner {
                    format!("Robot {} Wins!", winner_id)
                } else {
                    "Draw!".to_string()
                }
            );
        }
        info!("Exiting Bot Arena.");

        // After game over, show announcement and wait for ESC
        if game_ended {
            while !Renderer::window_should_close() {
                // Draw overlay with announcement
                renderer.draw_frame(
                    &self.arena,
                    &self.robots,
                    &self.particle_system,
                    self.current_turn,
                    self.max_turns,
                    self.current_cycle,
                    config::CYCLES_PER_TURN,
                    self.time_accumulator,
                    self.cycle_duration,
                    announcement.as_deref(),
                );
                if Renderer::is_key_down(macroquad::prelude::KeyCode::Escape) {
                    break;
                }
                next_frame().await;
            }
        }
        Ok(())
    }

    /// Update the simulation state for one fixed time step
    fn update_simulation(&mut self) {
        // Update previous state
        for robot in self.robots.iter_mut() {
            robot.update_prev_state();
        }
        self.particle_system.update_prev_state();

        let mut command_queue: VecDeque<ArenaCommand> = VecDeque::new();

        // Update Phase 1: Robot Processing & VM Execution
        // Process robot cycle updates (physics, power regen etc.)
        for robot in self.robots.iter_mut() {
            robot.process_cycle_updates(&self.arena);
        }

        // Update robots' area of interest (AOI)
        self.arena.update_all_robots_aoi(&mut self.robots);

        // Get all robot IDs once
        let robot_ids: Vec<u32> = self.robots.iter().map(|robot| robot.id).collect();

        // Collect robot information ahead of time to avoid borrow checker issues
        let robot_info: HashMap<u32, (Point, RobotStatus)> = self
            .robots
            .iter()
            .map(|robot| (robot.id, (robot.position, robot.status)))
            .collect();

        // Execute VM cycle for each robot
        for i in 0..self.robots.len() {
            let robot = &mut self.robots[i];

            // Update VM registers before execution
            robot.update_vm_state_registers(&self.arena);

            // Execute if not destroyed
            if robot.status != RobotStatus::Destroyed {
                // Store robot's properties locally to avoid borrowing issues
                let robot_id = robot.id;
                let robot_position = robot.position;
                let robot_status = robot.status;

                // Create closures
                let get_robot_ids = || robot_ids.clone();
                let mut get_robot_info = |id: u32| -> Option<(Point, RobotStatus)> {
                    if id == robot_id {
                        // For current robot, use up-to-date state
                        Some((robot_position, robot_status))
                    } else {
                        // For other robots, use the precomputed information
                        robot_info.get(&id).copied()
                    }
                };

                // Use our new method with the closures
                robot.execute_vm_cycle_with_provider(
                    get_robot_ids,
                    &mut get_robot_info,
                    &self.arena,
                    &mut command_queue,
                );
            }
        }

        // Update Phase 2: Command Execution
        // Spawn projectiles and effects fired this cycle
        for command in command_queue.drain(..) {
            match command {
                ArenaCommand::SpawnProjectile(projectile) => {
                    self.arena.spawn_projectile(projectile);
                }
                ArenaCommand::SpawnMuzzleFlash {
                    position,
                    direction,
                } => {
                    // Calculate muzzle flash position at tip of turret
                    let flash_offset_distance = config::UNIT_SIZE * 0.8;
                    let angle_rad = direction.to_radians();
                    let flash_offset_x = angle_rad.cos() * flash_offset_distance;
                    let flash_offset_y = angle_rad.sin() * flash_offset_distance;
                    let flash_pos_world = Vec2 {
                        x: (position.x + flash_offset_x) as f32,
                        y: (position.y + flash_offset_y) as f32,
                    };
                    self.particle_system
                        .spawn_muzzle_flash(flash_pos_world, direction);
                }
            }
        }

        // Update Phase 3: Arena Updates
        self.arena
            .update_projectiles(&mut self.robots, &mut self.particle_system);
        self.particle_system.update(self.cycle_duration);

        // --- New: Remove destroyed robots, add obstacles, check win/draw ---
        let destroyed_robots: Vec<Robot> = self
            .robots
            .iter()
            .filter(|r| r.status == RobotStatus::Destroyed)
            .cloned()
            .collect();
        for robot in &destroyed_robots {
            self.arena.add_obstacle_at_robot(robot);
        }
        // Remove destroyed robots from the robots vector
        self.robots.retain(|r| r.status != RobotStatus::Destroyed);

        // Check for win/draw
        let alive_robots: Vec<&Robot> = self
            .robots
            .iter()
            .filter(|r| r.status != RobotStatus::Destroyed)
            .collect();
        if alive_robots.len() == 1 {
            self.game_over = true;
            self.winner = Some(alive_robots[0].id);
        } else if alive_robots.is_empty() {
            self.game_over = true;
            self.winner = None;
        }
        // --- End new logic ---

        // Cycle/Turn Increment
        self.current_cycle += 1;
        if self.current_cycle >= config::CYCLES_PER_TURN {
            self.current_cycle = 0;
            self.current_turn += 1;

            // Update turn number in VM state for all robots
            for robot in self.robots.iter_mut() {
                robot.vm_state.turn = self.current_turn;
                robot.vm_state.cycle = self.current_cycle;
            }
        } else {
            // Update cycle number in VM state for all robots
            for robot in self.robots.iter_mut() {
                robot.vm_state.cycle = self.current_cycle;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::robot::{Robot, RobotStatus};
    use crate::types::Point;

    // Helper to create a dummy robot with a given id, position, and status
    fn dummy_robot(id: u32, pos: Point, status: RobotStatus) -> Robot {
        let mut robot = Robot::new(id, pos);
        robot.status = status;
        robot
    }

    #[test]
    fn test_destroyed_robot_removal_and_obstacle_placement() {
        let mut game = Game {
            arena: Arena::new(),
            robots: vec![dummy_robot(1, Point { x: 0.1, y: 0.1 }, RobotStatus::Active),
                         dummy_robot(2, Point { x: 0.2, y: 0.2 }, RobotStatus::Destroyed)],
            particle_system: ParticleSystem::new(),
            current_turn: 1,
            current_cycle: 0,
            max_turns: 10,
            time_accumulator: 0.0,
            cycle_duration: 1.0,
            game_over: false,
            winner: None,
        };
        // Before update: 2 robots, 0 obstacles
        assert_eq!(game.robots.len(), 2);
        assert_eq!(game.arena.obstacles.len(), 0);
        // Run update_simulation (should remove destroyed robot and add obstacle)
        game.update_simulation();
        // After update: 1 robot, 1 obstacle
        assert_eq!(game.robots.len(), 1);
        assert_eq!(game.arena.obstacles.len(), 1);
        // Obstacle should be at destroyed robot's position
        let obs_pos = game.arena.obstacles[0].position;
        assert!((obs_pos.x - 0.2).abs() < 1e-9 && (obs_pos.y - 0.2).abs() < 1e-9);
    }

    #[test]
    fn test_win_and_draw_logic() {
        // Test win condition: one robot left
        let mut game = Game {
            arena: Arena::new(),
            robots: vec![dummy_robot(1, Point { x: 0.1, y: 0.1 }, RobotStatus::Active)],
            particle_system: ParticleSystem::new(),
            current_turn: 1,
            current_cycle: 0,
            max_turns: 10,
            time_accumulator: 0.0,
            cycle_duration: 1.0,
            game_over: false,
            winner: None,
        };
        game.update_simulation();
        assert!(game.game_over);
        assert_eq!(game.winner, Some(1));

        // Test draw condition: no robots left
        let mut game = Game {
            arena: Arena::new(),
            robots: vec![],
            particle_system: ParticleSystem::new(),
            current_turn: 1,
            current_cycle: 0,
            max_turns: 10,
            time_accumulator: 0.0,
            cycle_duration: 1.0,
            game_over: false,
            winner: None,
        };
        game.update_simulation();
        assert!(game.game_over);
        assert_eq!(game.winner, None);
    }
}
