use crate::arena::Arena;
use crate::config;
use crate::types::*;
use crate::vm;
use crate::vm::instruction::Instruction;
use crate::vm::parser;
use crate::vm::state::VMState;
use rand::prelude::*;
use std::collections::VecDeque;
use std::f64::INFINITY;
use std::f64::consts::PI;
use crate::types::Scanner;

// Represents the possible states of a robot
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RobotStatus {
    Idle, // Just loaded, hasn't run yet
    Active,
    Stunned(u32), // Stores remaining stun duration in cycles
    Destroyed,
}

// Represents the Drive component of a robot
#[derive(Debug, Clone, Copy)]
pub struct DriveComponent {
    pub direction: f64,        // Current direction in degrees
    pub velocity: f64,         // Current velocity in units/cycle (+forward, -backward)
    pub pending_rotation: f64, // Degrees remaining to rotate
}

impl Default for DriveComponent {
    fn default() -> Self {
        DriveComponent {
            direction: 0.0,
            velocity: 0.0,
            pending_rotation: 0.0,
        }
    }
}

// Represents the Turret component of a robot
#[derive(Debug, Clone, Copy)]
pub struct TurretComponent {
    pub direction: f64,        // Absolute angle (0-359.9 degrees) relative to arena
    pub pending_rotation: f64, // Degrees remaining to rotate
    pub scanner: Scanner,      // Mounted scanner for target detection
    pub ranged: RangedWeapon,  // Mounted ranged weapon
}

impl Default for TurretComponent {
    fn default() -> Self {
        TurretComponent {
            direction: 0.0,
            pending_rotation: 0.0,
            scanner: Scanner::default(),
            ranged: RangedWeapon::default(),
        }
    }
}

// Represents a robot in the arena
#[derive(Debug, Clone)]
pub struct Robot {
    pub id: u32, // Unique identifier
    pub name: String, // Name derived from filename
    pub position: Point,
    pub prev_position: Point, // <-- Add previous position
    pub health: f64,
    pub power: f64,
    pub status: RobotStatus,
    pub drive: DriveComponent,
    pub prev_drive_direction: f64, // <-- Add previous drive direction
    pub turret: TurretComponent,
    pub prev_turret_direction: f64, // <-- Add previous turret direction
    pub vm_state: VMState,          // Made public for executor access
    pub program: Vec<Instruction>,
    pub rng: ThreadRng,
    pub aoi: Vec<u32>, // Area of interest - IDs of nearby robots
}

impl Robot {
    // Creates a new robot with default values at a given position
    pub fn new(id: u32, name: String, position: Point, center: Point) -> Self {
        // Calculate angle towards the center
        let dx = center.x - position.x;
        let dy = center.y - position.y;
        let angle_rad = dy.atan2(dx);
        let initial_direction_deg = angle_rad.to_degrees().rem_euclid(360.0);

        Robot {
            id,
            name, // Store the provided name
            position,
            prev_position: position,
            health: config::DEFAULT_INITIAL_HEALTH,
            power: config::DEFAULT_INITIAL_POWER,
            status: RobotStatus::Idle,
            drive: DriveComponent {
                direction: initial_direction_deg, // Set initial direction
                velocity: 0.0,
                pending_rotation: 0.0,
            },
            prev_drive_direction: initial_direction_deg, // Initialize prev state
            turret: TurretComponent {
                direction: initial_direction_deg, // Set initial direction
                pending_rotation: 0.0,
                scanner: Scanner::default(),
                ranged: RangedWeapon::default(),
            },
            prev_turret_direction: initial_direction_deg, // Initialize prev state
            vm_state: VMState::new(),
            program: Vec::new(), // Initialize empty program
            rng: thread_rng(),
            aoi: Vec::new(), // Initialize empty area of interest
        }
    }

    /// Updates the previous state fields with the current state.
    /// Should be called AFTER all simulation updates for the cycle are done.
    pub fn update_prev_state(&mut self) {
        self.prev_position = self.position;
        self.prev_drive_direction = self.drive.direction;
        self.prev_turret_direction = self.turret.direction;
    }

    /// Fires the ranged weapon with the specified power level, consuming power.
    /// Returns the projectile if successfully fired, otherwise None.
    pub fn fire_weapon(&mut self, requested_power: f64) -> Option<Projectile> {
        // Clamp requested power to valid range [0, 1]
        let clamped_power = requested_power.clamp(0.0, 1.0);
        // Determine actual power used based on available power
        let actual_power = clamped_power.min(self.power);

        if actual_power <= 0.0 {
            crate::debug_weapon!(
                self.id,
                self.vm_state.turn,
                self.vm_state.cycle,
                "Attempted to fire with insufficient power ({:.4})",
                actual_power
            );
            // TODO: Consider setting a VM fault?
            return None;
        }

        // Consume power
        self.power -= actual_power;

        // Calculate starting position from the *tip* of the turret line (80% radius)
        let start_offset_distance = config::UNIT_SIZE * 0.8; // Match visual turret line length
        let angle_rad = self.turret.direction.to_radians();
        let start_offset_x = angle_rad.cos() * start_offset_distance;
        let start_offset_y = angle_rad.sin() * start_offset_distance;
        let start_pos = Point {
            x: self.position.x + start_offset_x,
            y: self.position.y + start_offset_y,
        };

        // Create new projectile
        let projectile = Projectile {
            position: start_pos,      // Start 1 unit away
            prev_position: start_pos, // Initialize prev_position
            direction: self.turret.direction,
            // Speed is now constant, not scaled by power
            speed: self.turret.ranged.projectile_speed, // Use base speed directly
            power: actual_power, // Store power used for damage calculation later
            base_damage: self.turret.ranged.base_damage, // Get base damage from weapon
            source_robot: self.id,
        };

        crate::debug_weapon!(
            self.id,
            self.vm_state.turn,
            self.vm_state.cycle,
            "Fired projectile (Power: {:.2}, Speed: {:.2}, Remaining: {:.2})",
            actual_power,
            projectile.speed,
            self.power
        );

        // Return the created projectile
        Some(projectile)
    }

    /// New method to scan for targets using a function to get robot information by ID.
    /// This avoids the need to clone the entire robots array.
    pub fn scan_for_targets_by_id<F>(
        &self,
        get_robot_info: &mut F,
        robot_ids: &[u32],
        arena: &Arena,
    ) -> (f64, f64)
    where
        F: FnMut(u32) -> Option<(Point, RobotStatus)>,
    {
        // Setup scanning variables
        let scanner_pos = self.position;
        let scanner_dir_rad = self.turret.direction.to_radians();
        let scan_fov_half_rad = (self.turret.scanner.fov / 2.0).to_radians();
        let mut closest_target_dist_sq = INFINITY;
        let mut target_found = false;
        let mut best_target_angle_deg = 0.0;
        let mut best_target_dist = 0.0;

        // Scan through robot IDs
        for &other_id in robot_ids {
            if other_id == self.id {
                continue; // Don't scan self
            }

            // Get position and status information using the provided closure
            if let Some((target_pos, status)) = get_robot_info(other_id) {
                if status == RobotStatus::Destroyed {
                    continue; // Skip destroyed robots
                }

                let dx = target_pos.x - scanner_pos.x;
                let dy = target_pos.y - scanner_pos.y;
                let dist_sq = dx * dx + dy * dy;

                // 1. Check if within range (using squared distances)
                if dist_sq <= closest_target_dist_sq {
                    // 2. Calculate angle to target
                    let angle_to_target_rad = dy.atan2(dx);
                    let angle_to_target_deg_normalized =
                        angle_to_target_rad.to_degrees().rem_euclid(360.0);

                    // 3. Calculate angular difference and check FOV
                    let mut angle_diff = angle_to_target_rad - scanner_dir_rad;
                    // Normalize angle difference to [-PI, PI]
                    angle_diff = (angle_diff + PI) % (2.0 * PI) - PI;

                    if angle_diff.abs() <= scan_fov_half_rad {
                        // 4. Check Line-of-Sight (LOS) using arena collision check
                        let collision_dist = arena
                            .distance_to_collision(scanner_pos, angle_to_target_deg_normalized);
                        let target_dist = dist_sq.sqrt();

                        // If the distance to the target is less than the distance to a collision point, LOS is clear.
                        if target_dist < collision_dist - 1e-6 {
                            // Found a valid target closer than the previous best
                            closest_target_dist_sq = dist_sq;
                            target_found = true;
                            best_target_angle_deg = angle_to_target_deg_normalized;
                            best_target_dist = target_dist;
                        }
                    }
                }
            }
        }

        // Return results directly
        if target_found {
            (best_target_dist, best_target_angle_deg)
        } else {
            (0.0, 0.0) // Return 0.0 distance and 0.0 angle if no target found
        }
    }

    /// Loads a pre-parsed robot assembly program
    pub fn load_program(&mut self, program: parser::ParsedProgram) {
        // Store the instructions
        self.program = program.instructions;
        // Labels are handled by the parser and resolved to indices,
        // so we don't need to store program.labels here unless needed for debugging.

        // Reset VM state for the new program
        self.vm_state = VMState::new();

        // Program loaded, robot is ready (or Idle until first update)
        self.status = RobotStatus::Idle;
    }

    // Main update logic called once per cycle for the robot
    // Needs Arena reference for collision checks during movement processing
    pub fn update(&mut self, arena: &Arena) {
        // Process actions that occur automatically each cycle
        self.process_cycle_updates(arena);

        // Update VM state registers before executing instructions for this cycle
        self.update_vm_state_registers(arena);

        // TODO: Implement stun handling
        // REMOVED VM execution from here - handled by main loop
        /*
        if self.status == RobotStatus::Active {
            let _maybe_projectile = self.execute_vm_cycle();
        }
        */
    }

    /// Updates the read-only registers in the VM state before each VM cycle execution
    pub fn update_vm_state_registers(&mut self, arena: &Arena) {
        // Update @rand register
        let random_value = self.rng.r#gen::<f64>(); // <-- Fix gen call

        // Calculate forward/backward distances
        let forward_angle = self.drive.direction;
        let backward_angle = (self.drive.direction + 180.0).rem_euclid(360.0);
        let forward_dist = arena.distance_to_collision(self.position, forward_angle);
        let backward_dist = arena.distance_to_collision(self.position, backward_angle);

        let registers = &mut self.vm_state.registers;
        // Use .set_internal() for read-only registers
        registers
            .set_internal(vm::registers::Register::Turn, self.vm_state.turn as f64)
            .unwrap();
        registers
            .set_internal(vm::registers::Register::Cycle, self.vm_state.cycle as f64)
            .unwrap();
        registers
            .set_internal(vm::registers::Register::Rand, random_value)
            .unwrap();
        registers
            .set_internal(vm::registers::Register::Health, self.health)
            .unwrap();
        registers
            .set_internal(vm::registers::Register::Power, self.power)
            .unwrap();
        // @component is set by Select instruction
        registers
            .set_internal(
                vm::registers::Register::TurretDirection,
                self.turret.direction,
            )
            .unwrap();
        registers
            .set_internal(
                vm::registers::Register::DriveDirection,
                self.drive.direction,
            )
            .unwrap();
        registers
            .set_internal(vm::registers::Register::DriveVelocity, self.drive.velocity)
            .unwrap();
        registers
            .set_internal(vm::registers::Register::PosX, self.position.x)
            .unwrap();
        registers
            .set_internal(vm::registers::Register::PosY, self.position.y)
            .unwrap();
        registers
            .set_internal(vm::registers::Register::ForwardDistance, forward_dist)
            .unwrap();
        registers
            .set_internal(vm::registers::Register::BackwardDistance, backward_dist)
            .unwrap();
        // Weapon related registers
        registers
            .set_internal(vm::registers::Register::WeaponPower, self.power)
            .unwrap(); // Example: Use robot power
        registers
            .set_internal(vm::registers::Register::WeaponCooldown, 0.0)
            .unwrap(); // Placeholder
        // REMOVED ScanDistance and ScanAngle updates - handled by Scan instruction now
        // registers.set_internal(vm::registers::Register::ScanDistance, self.turret.scanner.last_scan_distance).unwrap();
        // registers.set_internal(vm::registers::Register::ScanAngle, self.turret.scanner.last_scan_angle).unwrap();
    }

    /// Execute one simulation cycle's worth of VM instructions.
    /// Requires the context of all robots and the arena state.
    pub fn execute_vm_cycle(
        &mut self,
        all_robots: &[Robot],
        arena: &Arena,
        command_queue: &mut VecDeque<ArenaCommand>,
    ) {
        // Transition from Idle to Active *before* the guard check
        if self.status == RobotStatus::Idle {
            self.status = RobotStatus::Active;
        }

        // Now check if we should execute based on the potentially updated status
        if self.status != RobotStatus::Active || self.vm_state.fault.is_some() {
            return; // Don't execute if not active or already faulted
        }

        // --- Check if waiting for multi-cycle instruction ---
        if self.vm_state.instruction_cycles_remaining > 0 {
            self.vm_state.instruction_cycles_remaining -= 1;
            return; // Instruction still executing, do nothing else this cycle
        }

        // Create instruction executor
        let executor = vm::executor::InstructionExecutor::new();

        let ip = self.vm_state.ip;
        let mut spent = 0;

        while spent < 1 {
            // --- Get and Execute Instruction ---
            if let Some(instr) = self.program.get(ip).cloned() {
                // Get the current instruction location for debugging
                if ip < self.program.len() {
                    let instr_str = format!("{:?}", instr);
                    crate::debug_instructions!(
                        self.id,
                        self.vm_state.turn,
                        self.vm_state.cycle,
                        "Executing instruction at IP {}: {}",
                        ip,
                        instr_str
                    );

                    if instr_str.contains("Rotate") {
                        crate::debug_instructions!(
                            self.id,
                            self.vm_state.turn,
                            self.vm_state.cycle,
                            "ROTATE INSTRUCTION: executing rotate"
                        );
                    } else if instr_str.contains("Drive") {
                        crate::debug_instructions!(
                            self.id,
                            self.vm_state.turn,
                            self.vm_state.cycle,
                            "DRIVE INSTRUCTION: executing drive"
                        );
                    }
                }

                // Calculate cost BEFORE execution (needed for Rotate cost)
                let cost = instr.cycle_cost(&self.vm_state);
                spent += cost;

                // Store initial IP in case instruction doesn't modify it (e.g., jumps)
                let ip_before_exec = self.vm_state.ip;

                // Execute the instruction, passing the necessary context
                match executor.execute_instruction(self, all_robots, arena, &instr, command_queue) {
                    // Pass all_robots and arena
                    Ok(()) => {
                        // Instruction succeeded
                        // If the instruction pointer wasn't changed by a jump/call,
                        // advance it to the next instruction for the *next* cycle.
                        if self.vm_state.ip == ip_before_exec {
                            self.vm_state.advance_ip();
                        }
                        // Set remaining cycles for the *next* cycles
                        if cost > 0 {
                            self.vm_state.instruction_cycles_remaining = cost - 1;
                        } else {
                            // Should not happen with current costs, but defensively set to 0
                            self.vm_state.instruction_cycles_remaining = 0;
                        }
                    }
                    Err(fault) => {
                        // Instruction failed
                        crate::debug_vm!(
                            self.id,
                            self.vm_state.turn,
                            self.vm_state.cycle,
                            "VM Fault at IP {}: {:?} ({:?})",
                            ip,
                            fault,
                            instr
                        );
                        self.vm_state.set_fault(fault);
                        // TODO: Attempt jump to :fault label if it exists, otherwise halt/disable robot
                        // For now, just halt by setting remaining cycles high?
                        self.vm_state.instruction_cycles_remaining = u32::MAX; // Effectively halts
                    }
                }
            } else {
                // End of program reached or invalid IP
                // Halt execution by setting remaining cycles high?
                self.vm_state.instruction_cycles_remaining = u32::MAX; // Effectively halts
            }
        }

        // --- End of instructions for this cycle ---

        // --- Debug Output ---
        crate::debug_instructions!(
            self.id,
            self.vm_state.turn,
            self.vm_state.cycle,
            "-- Cycle End --"
        );
        // Print registers with names
        use crate::vm::registers::Register::*; // Import variants for easier access
        let all_regs = [
            D1,
            D2,
            D3,
            D4,
            D5,
            D6,
            D7,
            D8,
            D9,
            C,
            Result,
            Fault,
            Turn,
            Cycle,
            Rand,
            Health,
            Power,
            Component,
            TurretDirection,
            DriveDirection,
            DriveVelocity,
            PosX,
            PosY,
            ForwardDistance,
            BackwardDistance,
            WeaponPower,
            WeaponCooldown,
            TargetDistance,
            TargetDirection,
        ];
        for reg in all_regs.iter() {
            match self.vm_state.registers.get(*reg) {
                Ok(value) => crate::debug_instructions!(
                    self.id,
                    self.vm_state.turn,
                    self.vm_state.cycle,
                    "{:?}: {:.4}",
                    reg,
                    value
                ),
                Err(e) => crate::debug_instructions!(
                    self.id,
                    self.vm_state.turn,
                    self.vm_state.cycle,
                    "{:?}: Error getting value: {:?}",
                    reg,
                    e
                ),
            }
        }

        // Use the new Stack::view() method
        crate::debug_instructions!(
            self.id,
            self.vm_state.turn,
            self.vm_state.cycle,
            "Stack (top first): {:?}",
            self.vm_state.stack.view().iter().rev().collect::<Vec<_>>()
        );
        crate::debug_instructions!(
            self.id,
            self.vm_state.turn,
            self.vm_state.cycle,
            "IP: {}, Cycles Left: {}, Fault: {:?}",
            self.vm_state.ip,
            self.vm_state.instruction_cycles_remaining,
            self.vm_state.fault
        );
        // --- End Debug ---
    }

    /// A version of execute_vm_cycle that uses a robot info provider function to avoid cloning
    pub fn execute_vm_cycle_with_provider<F, G>(
        &mut self,
        get_robot_ids: F,
        get_robot_info: &mut G,
        arena: &Arena,
        command_queue: &mut VecDeque<ArenaCommand>,
    ) -> Option<vm::error::VMFault>
    where
        F: Fn() -> Vec<u32>,
        G: FnMut(u32) -> Option<(Point, RobotStatus)>,
    {
        use log::debug;

        // Transition from Idle to Active *before* the guard check
        if self.status == RobotStatus::Idle {
            debug!("Robot {}: Transitioning from Idle to Active", self.id);
            self.status = RobotStatus::Active;
        }

        // Now check if we should execute based on the potentially updated status
        if self.status != RobotStatus::Active || self.vm_state.fault.is_some() {
            return self.vm_state.fault; // Don't execute if not active or already faulted
        }

        // --- Check if waiting for multi-cycle instruction ---
        if self.vm_state.instruction_cycles_remaining > 0 {
            self.vm_state.instruction_cycles_remaining -= 1;
            return None; // Instruction still executing, do nothing else this cycle
        }

        // Create instruction executor
        let executor = vm::executor::InstructionExecutor::new();

        let robot_ids = get_robot_ids();
        let ip = self.vm_state.ip;
        let mut spent = 0;

        while spent < 1 {
            if let Some(instr) = self.program.get(ip).cloned() {
                // Get the current instruction location for debugging
                if ip < self.program.len() {
                    let instr_str = format!("{:?}", instr);
                    debug!(
                        "Robot {} executing instruction at IP {}: {}",
                        self.id, ip, instr_str
                    );
                }

                // Calculate cost BEFORE execution
                let cost = instr.cycle_cost(&self.vm_state);
                spent += cost;

                // Store initial IP in case instruction doesn't modify it
                let ip_before_exec = self.vm_state.ip;

                // Execute using the executor for all instructions
                let result = executor.execute_instruction_by_id(
                    self,
                    get_robot_info,
                    &robot_ids,
                    arena,
                    &instr,
                    command_queue,
                );

                match result {
                    Ok(()) => {
                        // Instruction succeeded
                        // If the instruction pointer wasn't changed by a jump/call,
                        // advance it to the next instruction for the *next* cycle.
                        if self.vm_state.ip == ip_before_exec {
                            self.vm_state.advance_ip();
                        }
                        // Set remaining cycles for the *next* cycles
                        if cost > 0 {
                            self.vm_state.instruction_cycles_remaining = cost - 1;
                        } else {
                            self.vm_state.instruction_cycles_remaining = 0;
                        }
                    }
                    Err(fault) => {
                        // Instruction failed
                        debug!("Robot {} VM Fault at IP {}: {:?}", self.id, ip, fault);
                        self.vm_state.set_fault(fault);
                        self.vm_state.instruction_cycles_remaining = u32::MAX; // Effectively halts
                        return Some(fault);
                    }
                }
            } else {
                // End of program reached or invalid IP
                self.vm_state.instruction_cycles_remaining = u32::MAX; // Effectively halts
                break;
            }
        }

        None // No fault occurred
    }

    // --- Component Control Methods ---

    // Sets the target velocity for the drive component
    pub fn set_drive_velocity(&mut self, velocity: f64) {
        // Velocity is in coordinate units per cycle
        crate::debug_drive!(
            self.id,
            self.vm_state.turn,
            self.vm_state.cycle,
            "set_drive_velocity: Received velocity = {:.4} coordinate units per cycle ({:.4} units per turn)",
            velocity,
            velocity * config::CYCLES_PER_TURN as f64 / config::UNIT_SIZE
        );

        self.drive.velocity = velocity;

        crate::debug_drive!(
            self.id,
            self.vm_state.turn,
            self.vm_state.cycle,
            "set_drive_velocity: velocity is now = {:.4} units per cycle",
            self.drive.velocity
        );
    }

    // Requests a relative rotation for the drive component
    pub fn request_drive_rotation(&mut self, angle_delta: f64) {
        // Accumulate requested rotation. Actual rotation happens in `update`.
        let adjusted = self.drive.pending_rotation + angle_delta;
        crate::debug_drive!(
            self.id,
            self.vm_state.turn,
            self.vm_state.cycle,
            "request_drive_rotation: delta = {:.2}, pending = {:.2}, current = {:.2}, adj: {:.2}",
            angle_delta,
            self.drive.pending_rotation,
            self.drive.direction,
            adjusted,
        );
        self.drive.pending_rotation = adjusted;
    }

    // Requests a relative rotation for the turret component
    pub fn request_turret_rotation(&mut self, angle_delta: f64) {
        // Accumulate requested rotation. Actual rotation happens in `update`.
        let adjusted = self.drive.pending_rotation + angle_delta;
        crate::debug_weapon!(
            self.id,
            self.vm_state.turn,
            self.vm_state.cycle,
            "request_turret_rotation: delta {:.2}, pending: {:.2}, current: {:.2}, adj: {:.2}",
            angle_delta,
            self.turret.pending_rotation,
            self.turret.direction,
            adjusted,
        );
        self.turret.pending_rotation = adjusted;
    }

    // --- Internal Update Helpers (to be called from update()) ---

    // Processes actions that resolve over time (like rotation)
    // Called once per cycle from update()
    // Needs Arena reference for collision checks during movement processing
    pub fn process_cycle_updates(&mut self, arena: &Arena) {
        // --- Power Regeneration ---
        self.power = (self.power + config::POWER_REGEN_RATE).min(1.0);

        // --- Process Rotations ---
        let max_rot = config::MAX_ROTATION_PER_CYCLE;

        // Process Drive Rotation
        if self.drive.pending_rotation.abs() > 1e-6 {
            // Use epsilon comparison
            let drive_rot_this_cycle = self.drive.pending_rotation.clamp(-max_rot, max_rot);
            let old_dir = self.drive.direction;
            self.drive.direction = (self.drive.direction + drive_rot_this_cycle).rem_euclid(360.0);
            self.drive.pending_rotation -= drive_rot_this_cycle;
            crate::debug_drive!(
                self.id,
                self.vm_state.turn,
                self.vm_state.cycle,
                "Rotated by {:.2} (pending now {:.2}). Direction {:.1} -> {:.1}",
                drive_rot_this_cycle,
                self.drive.pending_rotation,
                old_dir,
                self.drive.direction
            );
        } else if self.drive.pending_rotation != 0.0 {
            // Clear tiny pending rotations
            self.drive.pending_rotation = 0.0;
        }

        // Process Turret Rotation
        if self.turret.pending_rotation.abs() > 1e-6 {
            // Use epsilon comparison
            let turret_rot_this_cycle = self.turret.pending_rotation.clamp(-max_rot, max_rot);
            let old_dir = self.turret.direction;
            self.turret.direction =
                (self.turret.direction + turret_rot_this_cycle).rem_euclid(360.0);
            self.turret.pending_rotation -= turret_rot_this_cycle;
            crate::debug_weapon!(
                self.id,
                self.vm_state.turn,
                self.vm_state.cycle,
                "Rotated turret by {:.2} (pending now {:.2}). Direction {:.1} -> {:.1}",
                turret_rot_this_cycle,
                self.turret.pending_rotation,
                old_dir,
                self.turret.direction
            );
        } else if self.turret.pending_rotation != 0.0 {
            self.turret.pending_rotation = 0.0;
        }

        // --- Process Movement ---
        self.process_movement(arena);
    }

    // Processes movement based on velocity and checks for collisions
    fn process_movement(&mut self, arena: &Arena) {
        // DEBUG: Log velocity at start of movement processing
        crate::debug_drive!(
            self.id,
            self.vm_state.turn,
            self.vm_state.cycle,
            "Movement start. velocity={:.4} coordinate units per cycle, Direction={:.1}",
            self.drive.velocity,
            self.drive.direction
        );
        if self.drive.velocity.abs() < 1e-9 {
            return; // Not moving
        }

        // 1. Determine maximum safe travel distance for the EDGE in the current direction
        let max_safe_distance = arena.distance_to_collision(self.position, self.drive.direction);

        // 2. Calculate intended travel distance based on velocity (in coordinate units per cycle)
        let intended_distance = self.drive.velocity;

        // 3. Clamp the actual travel distance
        let actual_distance = if intended_distance > 0.0 {
            // Add a small buffer to avoid getting too close to walls/obstacles
            let safe_distance = max_safe_distance - config::UNIT_SIZE * 0.01;
            if safe_distance <= 0.0 {
                // Already at or very close to a collision
                0.0
            } else {
                // Move forward the intended distance or until near the obstacle
                intended_distance.min(safe_distance)
            }
        } else {
            // Moving backward
            intended_distance // Allow full backward movement for now
        };

        // DEBUG: Log calculated distances
        crate::debug_drive!(
            self.id,
            self.vm_state.turn,
            self.vm_state.cycle,
            "IntendedDist={:.4}, MaxSafeDist={:.4}, ActualDist={:.4} coordinate units per cycle",
            intended_distance,
            max_safe_distance,
            actual_distance
        );

        // If clamped distance is effectively zero, stop velocity and exit.
        if actual_distance.abs() < 1e-9 {
            self.drive.velocity = 0.0;
            return;
        }

        // 4. Calculate movement vector using the clamped distance
        let angle_rad = self.drive.direction.to_radians();
        let dx = angle_rad.cos() * actual_distance;
        let dy = angle_rad.sin() * actual_distance;

        let next_pos = Point {
            x: self.position.x + dx,
            y: self.position.y + dy,
        };
        crate::debug_drive!(
            self.id,
            self.vm_state.turn,
            self.vm_state.cycle,
            "Moving by ({:.4}, {:.4}) from ({:.3},{:.3}) to ({:.3},{:.3})",
            dx,
            dy,
            self.position.x,
            self.position.y,
            next_pos.x,
            next_pos.y
        );

        // 5. Update Position
        self.position = next_pos;

        // Log new position after movement
        crate::debug_drive!(
            self.id,
            self.vm_state.turn,
            self.vm_state.cycle,
            "Position after move: ({:.3}, {:.3})",
            self.position.x,
            self.position.y
        );

        // 6. Check if we've hit a boundary or obstacle after the move
        if self.position.x < 0.0
            || self.position.x > arena.width
            || self.position.y < 0.0
            || self.position.y > arena.height
        {
            crate::debug_drive!(
                self.id,
                self.vm_state.turn,
                self.vm_state.cycle,
                "Boundary collision AFTER movement clamp! Adjusting position."
            );
            self.position.x = self.position.x.clamp(0.0, arena.width);
            self.position.y = self.position.y.clamp(0.0, arena.height);
            self.drive.velocity = 0.0; // Stop the robot
        }
        if arena.check_collision(self.position) {
            // Check current position
            crate::debug_drive!(
                self.id,
                self.vm_state.turn,
                self.vm_state.cycle,
                "Obstacle collision AFTER movement clamp! Stopping."
            );
            self.drive.velocity = 0.0; // Stop the robot
        }
    }

    // Add this helper function
    pub fn get_current_instruction_string(&self) -> String {
        if self.program.is_empty() || self.vm_state.ip >= self.program.len() {
            return "-".to_string(); // Or "Idle", "None"
        }

        let instruction = &self.program[self.vm_state.ip];
        format!("{:?}", instruction)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::Arena;
    use crate::types::ArenaCommand;
    // Import ArenaCommand
    use crate::types::Point;
    use crate::vm::parser::{ParsedProgram, parse_assembly};
    use crate::vm::registers::Register;
    use crate::vm::instruction::Instruction;
    use crate::vm::executor::Operand;
    use crate::vm::executor::processor::InstructionProcessor;

    // For float comparisons

    // Helper function to parse a program string
    fn parse_program(source: &str) -> ParsedProgram {
        parse_assembly(source, None)
            .unwrap_or_else(|_| panic!("Failed to parse program: {}", source))
    }

    // Helper to simulate a robot cycle
    fn simulate_cycle(
        robot: &mut Robot,
        all_robots: &[Robot],
        arena: &Arena,
        command_queue: &mut VecDeque<ArenaCommand>,
    ) {
        robot.vm_state.instruction_cycles_remaining = 0; // Reset for test
        robot.execute_vm_cycle(all_robots, arena, command_queue);
        robot.process_cycle_updates(arena);
    }

    // Helper to select a component
    #[allow(dead_code)]
    fn select_component(robot: &mut Robot, component_id: u8) {
        robot
            .vm_state
            .registers
            .set_internal(Register::Component, component_id as f64)
            .unwrap();
    }

    #[test]
    fn test_basic_movement() {
        // Create a larger arena with no obstacles for testing
        let mut arena = Arena::new();
        
        // Override with a 10x10 arena (10 times bigger than default)
        arena.width = 10.0;
        arena.height = 10.0;
        arena.grid_width = 200; // 10*20
        arena.grid_height = 200; // 10*20
        arena.obstacles.clear(); // Make sure there are no obstacles
        
        // Position the robot farther from the edge at (1.0, 1.0) to ensure it can move a full unit
        let center = Point { x: arena.width / 2.0, y: arena.height / 2.0 };
        let mut robot = Robot::new(0, String::new(), Point { x: 1.0, y: 1.0 }, center);
        let mut command_queue = VecDeque::new();

        // Print arena size
        println!("Arena size: width={}, height={}", arena.width, arena.height);
        println!("Arena grid: {}x{} units", arena.grid_width, arena.grid_height);
        println!("Unit size: {}", arena.unit_size);
        
        let program = parse_program(
            r#"
            select 1      ; select drive
            drive 1.0     ; set velocity to 1.0 grid unit per turn
            rotate 0.0    ; set rotation to 0 degrees (east)
        "#,
        );

        robot.load_program(program);

        // Execute the select instruction
        simulate_cycle(&mut robot, &[], &arena, &mut command_queue);

        // Execute the drive instruction
        simulate_cycle(&mut robot, &[], &arena, &mut command_queue);

        // Execute the rotate instruction 
        simulate_cycle(&mut robot, &[], &arena, &mut command_queue);

        // Explicitly set direction to 0 for this test, overriding center-facing default
        robot.drive.direction = 0.0;

        // Expected velocity is 1.0 * UNIT_SIZE / CYCLES_PER_TURN coordinate units per cycle
        let expected_velocity = config::UNIT_SIZE / config::CYCLES_PER_TURN as f64;
        
        // Debug print
        println!(
            "Before movement: velocity = {} coordinate units per cycle (expected {})", 
            robot.drive.velocity, 
            expected_velocity
        );
        
        // Check the expected collision distance at the start
        let start_collision_distance = arena.distance_to_collision(robot.position, robot.drive.direction);
        println!("Initial distance to collision: {} coordinate units", start_collision_distance);
        
        // Verify that the expected move distance is reasonable
        let expected_move_distance = expected_velocity * config::CYCLES_PER_TURN as f64;
        println!("Expected to move {} coordinate units in one turn ({} grid units)", 
             expected_move_distance, expected_move_distance / config::UNIT_SIZE);
        
        if start_collision_distance < expected_move_distance {
            println!("WARNING: Distance to collision ({}) is less than expected move distance ({}). Robot will stop early!", 
                start_collision_distance, expected_move_distance);
        }
        
        assert!(
            (robot.drive.velocity - expected_velocity).abs() < 1e-9, 
            "Drive velocity should be {}, but was {}",
            expected_velocity,
            robot.drive.velocity
        );

        // Now simulate a full turn (CYCLES_PER_TURN cycles) to verify movement
        let start_x = robot.position.x;
        
        // Track total distance moved
        let mut total_distance = 0.0;
        
        // Process movement for a full turn
        for i in 0..config::CYCLES_PER_TURN {
            let pos_before = robot.position.x;
            
            // Check if we're about to hit something
            if i % 10 == 0 || i == 99 {
                let remaining_dist = arena.distance_to_collision(robot.position, robot.drive.direction);
                println!("Cycle {}: Distance to collision: {} coordinate units", i, remaining_dist);
            }
            
            robot.process_cycle_updates(&arena);
            let distance_this_cycle = robot.position.x - pos_before;
            total_distance += distance_this_cycle;
            
            // Print some debug info every 10 cycles
            if i % 10 == 0 || i == config::CYCLES_PER_TURN - 1 {
                println!(
                    "Cycle {}: moved {} coord units this cycle, total so far = {} coord units ({} grid units), velocity = {}, position = ({}, {})", 
                    i, 
                    distance_this_cycle, 
                    total_distance,
                    total_distance / config::UNIT_SIZE,
                    robot.drive.velocity,
                    robot.position.x,
                    robot.position.y
                );
            }
        }

        // Check that the robot moved ~0.05 coordinate units (1 grid unit)
        let distance_moved = robot.position.x - start_x;
        println!("Final distance moved: {} coordinate units ({} grid units)", 
            distance_moved, distance_moved / config::UNIT_SIZE);
        
        // For the test, we'll check if the robot moved 1 grid unit (with a small tolerance)
        assert!(
            (distance_moved - config::UNIT_SIZE).abs() < 0.001,
            "Robot should move {} coordinate units (1 grid unit) per turn with drive 1.0, but moved {} coordinate units",
            config::UNIT_SIZE,
            distance_moved
        );
    }

    // Add another test to verify fractional movement
    #[test]
    fn test_fractional_movement() {
        // Create a larger arena with no obstacles for testing
        let mut arena = Arena::new();
        
        // Override with a 10x10 arena
        arena.width = 10.0;
        arena.height = 10.0;
        arena.grid_width = 200; // 10*20
        arena.grid_height = 200; // 10*20
        arena.obstacles.clear(); // Make sure there are no obstacles
        
        // Position the robot away from the edges
        let center = Point { x: arena.width / 2.0, y: arena.height / 2.0 };
        let mut robot = Robot::new(0, String::new(), Point { x: 1.0, y: 1.0 }, center);
        let mut command_queue = VecDeque::new();

        // First select drive component
        robot.vm_state.set_selected_component(1).unwrap();
        
        // Set velocity to 0.5 grid units per turn (using the Drive instruction directly)
        let drive_instruction = Instruction::Drive(Operand::Value(0.5));
        let processor = vm::executor::ComponentOperations::new();
        processor.process(&mut robot, &[], &arena, &drive_instruction, &mut command_queue).unwrap();
        
        // Expected velocity is 0.5 * UNIT_SIZE / CYCLES_PER_TURN coordinate units per cycle
        let expected_velocity = 0.5 * config::UNIT_SIZE / config::CYCLES_PER_TURN as f64;
        
        // Check if velocity was set correctly
        assert!(
            (robot.drive.velocity - expected_velocity).abs() < 1e-9,
            "Drive velocity should be {}, but was {}",
            expected_velocity,
            robot.drive.velocity
        );
        
        // Set direction to east (0 degrees)
        robot.drive.direction = 0.0;
        
        // Now simulate a full turn (CYCLES_PER_TURN cycles) to verify movement
        let start_x = robot.position.x;
        
        // Process movement for a full turn
        for _ in 0..config::CYCLES_PER_TURN {
            robot.process_cycle_updates(&arena);
        }

        // Check that the robot moved ~0.025 coordinate units (0.5 grid units)
        let distance_moved = robot.position.x - start_x;
        println!("Fractional test: moved {} coordinate units ({} grid units) with drive 0.5", 
            distance_moved, distance_moved / config::UNIT_SIZE);
        
        assert!(
            (distance_moved - 0.5 * config::UNIT_SIZE).abs() < 0.001,
            "Robot should move {} coordinate units (0.5 grid units) per turn with drive 0.5, but moved {} coordinate units",
            0.5 * config::UNIT_SIZE,
            distance_moved
        );
    }

    #[test]
    fn test_component_switching() {
        let mut robot = Robot::new(0, String::new(), Point { x: 0.5, y: 0.5 }, Point { x: 0.5, y: 0.5 });
        let arena = Arena::default();
        let mut command_queue = VecDeque::new();

        let program = parse_program(
            r#"
            select 1         ; select drive
            drive 0.5        ; set velocity
            select 2         ; select turret
            rotate 45.0      ; set turret rotation
        "#,
        );

        robot.load_program(program);

        // Execute select drive
        simulate_cycle(&mut robot, &[], &arena, &mut command_queue);

        // Execute drive instruction
        simulate_cycle(&mut robot, &[], &arena, &mut command_queue);

        assert!(
            robot.drive.velocity > 0.0,
            "Drive velocity should be set after drive instruction"
        );

        // Execute select turret
        simulate_cycle(&mut robot, &[], &arena, &mut command_queue);

        // Execute rotate turret
        simulate_cycle(&mut robot, &[], &arena, &mut command_queue);

        // Check if rotation was set
        assert_ne!(robot.turret.pending_rotation, 0.0, "Turret rotation should be set");

        // Verify no commands were queued
        crate::debug_robot!(
            robot.id,
            robot.vm_state.turn,
            robot.vm_state.cycle,
            "Command queue length: {}",
            command_queue.len()
        );
        assert!(
            command_queue.is_empty(),
            "No commands should be queued for these instructions"
        );
    }

    #[test]
    fn test_program_errors() {
        let mut robot = Robot::new(0, String::new(), Point { x: 0.5, y: 0.5 }, Point { x: 0.5, y: 0.5 });
        let arena = Arena::default();
        let mut command_queue = VecDeque::new();

        // Test parsing errors
        let result = parse_assembly("invalid instruction", None);
        assert!(result.is_err());

        // Test runtime errors (division by zero, etc.)
        let runtime_error_program = parse_program(
            r#"
            push 5.0
            push 0.0
            div       ; Division by zero error
        "#,
        );

        robot.load_program(runtime_error_program);

        // Execute the first two instructions (push 5.0, push 0.0)
        simulate_cycle(&mut robot, &[], &arena, &mut command_queue);
        simulate_cycle(&mut robot, &[], &arena, &mut command_queue);

        // Execute the div instruction which should cause a fault
        robot.vm_state.instruction_cycles_remaining = 0; // Reset for test
        robot.execute_vm_cycle(&[], &arena, &mut command_queue);

        // Now check for the fault
        assert!(
            robot.vm_state.fault.is_some(),
            "Expected VM fault for division by zero"
        );
    }

    #[test]
    fn test_register_interaction() {
        let mut robot = Robot::new(0, String::new(), Point { x: 0.5, y: 0.5 }, Point { x: 0.5, y: 0.5 });
        let arena = Arena::default();

        let program = parse_program(
            r#"
            mov @d0 123.0     ; Set scratch register
            mov @result @d0   ; Copy to result
            push @result      ; Push result to stack
        "#,
        );

        robot.load_program(program);
        simulate_cycle(&mut robot, &[], &arena, &mut VecDeque::new());
        simulate_cycle(&mut robot, &[], &arena, &mut VecDeque::new());
        simulate_cycle(&mut robot, &[], &arena, &mut VecDeque::new());

        // Top of stack should be 123.0
        let val = robot.vm_state.stack.pop().unwrap();
        assert_eq!(val, 123.0);
    }

    #[test]
    fn test_fire_weapon() {
        let arena = Arena::new();
        let center = Point { x: arena.width / 2.0, y: arena.height / 2.0 };
        let mut robot = Robot::new(0, "TestRobot".to_string(), Point { x: 0.5, y: 0.5 }, center);
        let mut command_queue = VecDeque::new();

        // Set up robot state
        robot.power = 0.5;

        let program = parse_program(
            r#"
            select 2          ; Select turret
            fire 0.5          ; Fire weapon with 0.5 power
        "#,
        );

        robot.load_program(program);

        // Execute the select instruction
        simulate_cycle(&mut robot, &[], &arena, &mut command_queue);

        // Execute the fire instruction
        simulate_cycle(&mut robot, &[], &arena, &mut command_queue);

        // Should've queued a command to spawn a projectile and a muzzle flash
        crate::debug_robot!(
            robot.id,
            robot.vm_state.turn,
            robot.vm_state.cycle,
            "Command queue length: {}",
            command_queue.len()
        );
        assert_eq!(
            command_queue.len(),
            2,
            "Expected 2 commands (SpawnProjectile and SpawnMuzzleFlash)"
        );

        // Verify projectile was created
        match command_queue.pop_front().unwrap() {
            ArenaCommand::SpawnProjectile(proj) => {
                assert_eq!(proj.source_robot, 0);
                assert!(
                    (proj.power - 0.5).abs() < 0.001,
                    "Projectile power should be ~0.5"
                );
            }
            _ => panic!("Expected SpawnProjectile command"),
        }

        // Verify muzzle flash was created
        match command_queue.pop_front().unwrap() {
            ArenaCommand::SpawnMuzzleFlash { .. } => { /* Success */ }
            _ => panic!("Expected SpawnMuzzleFlash command"),
        }
    }

    // ... other tests ...
}
