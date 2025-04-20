use crate::arena::Arena;
use crate::robot::{Robot, RobotStatus};
use crate::types::{ArenaCommand, Point};
use crate::vm::error::VMFault;
use crate::vm::registers::Register;
use std::collections::VecDeque;

use super::processor::InstructionProcessor;
use crate::vm::instruction::Instruction;

/// Processor for robot combat operations
pub struct CombatOperations;

impl CombatOperations {
    pub fn new() -> Self {
        CombatOperations
    }

    // Shared helper for firing
    fn handle_fire(
        robot: &mut Robot,
        power: f64,
        command_queue: &mut VecDeque<ArenaCommand>,
    ) {
        let fire_position = robot.position;
        let fire_direction = robot.turret.direction;
        if let Some(projectile) = robot.fire_weapon(power) {
            command_queue.push_back(ArenaCommand::SpawnProjectile(projectile));
            command_queue.push_back(ArenaCommand::SpawnMuzzleFlash {
                position: fire_position,
                direction: fire_direction,
            });
        }
    }

    // Shared helper for scanning
    fn handle_scan<F>(
        robot: &mut Robot,
        get_robot_info: &mut F,
        robot_ids: &[u32],
        arena: &Arena,
    ) -> Result<(f64, f64), VMFault>
    where
        F: FnMut(u32) -> Option<(Point, RobotStatus)>,
    {
        let (distance, angle) = robot.scan_for_targets_by_id(get_robot_info, robot_ids, arena);
        robot
            .vm_state
            .registers
            .set_internal(Register::TargetDistance, distance)
            .map_err(|_| VMFault::PermissionError)?;
        robot
            .vm_state
            .registers
            .set_internal(Register::TargetDirection, angle)
            .map_err(|_| VMFault::PermissionError)?;
        Ok((distance, angle))
    }
}

impl InstructionProcessor for CombatOperations {
    fn can_process(&self, instruction: &Instruction) -> bool {
        matches!(
            instruction,
            Instruction::Fire(_) | Instruction::Scan
        )
    }

    fn process(
        &self,
        robot: &mut Robot,
        all_robots: &[Robot],
        arena: &Arena,
        instruction: &Instruction,
        command_queue: &mut VecDeque<ArenaCommand>,
    ) -> Result<(), VMFault> {
        match instruction {
            Instruction::Fire(op) => {
                crate::debug_weapon!(robot.id, robot.vm_state.turn, robot.vm_state.cycle, "FIRE!");
                let power = op.get_value(&robot.vm_state)?;
                Self::handle_fire(robot, power, command_queue);
                Ok(())
            }
            Instruction::Scan => {
                // Build closure and robot_ids from all_robots
                let mut get_robot_info = |id: u32| {
                    for other_robot in all_robots {
                        if other_robot.id == id {
                            return Some((other_robot.position, other_robot.status));
                        }
                    }
                    None
                };
                let robot_ids: Vec<u32> = all_robots.iter().map(|r| r.id).collect();
                Self::handle_scan(robot, &mut get_robot_info, &robot_ids, arena)?;
                Ok(())
            }
            _ => Err(VMFault::InvalidInstruction),
        }
    }
}

/// A version of the process method that uses robot IDs and a provider function
/// This will be useful for execute_vm_cycle_with_provider
pub fn process_by_id<F>(
    robot: &mut Robot,
    get_robot_info: &mut F,
    robot_ids: &[u32],
    arena: &Arena,
    instruction: &Instruction,
    command_queue: &mut VecDeque<ArenaCommand>,
) -> Result<(), VMFault>
where
    F: FnMut(u32) -> Option<(Point, RobotStatus)>,
{
    let combat_ops = CombatOperations::new();
    match instruction {
        Instruction::Fire(op) => {
            crate::debug_weapon!(robot.id, robot.vm_state.turn, robot.vm_state.cycle, "FIRE!");
            let power = op.get_value(&robot.vm_state)?;
            CombatOperations::handle_fire(robot, power, command_queue);
            Ok(())
        }
        Instruction::Scan => {
            CombatOperations::handle_scan(robot, get_robot_info, robot_ids, arena)?;
            Ok(())
        }
        _ => {
            if combat_ops.can_process(instruction) {
                combat_ops.process(robot, &[], arena, instruction, command_queue)
            } else {
                Err(VMFault::InvalidInstruction)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::arena::Arena;
    use crate::robot::{Robot, RobotStatus};
    use crate::types::{ArenaCommand, Point};

    use crate::vm::executor::{CombatOperations, InstructionProcessor};
    use crate::vm::instruction::Instruction;
    use crate::vm::operand::Operand;
    use crate::vm::registers::Register;
    use std::collections::VecDeque;

    fn create_test_robot() -> Robot {
        let mut robot = Robot::new(1, Point { x: 0.5, y: 0.5 });
        robot.status = RobotStatus::Active;
        robot
    }

    fn create_test_robots() -> Vec<Robot> {
        vec![create_test_robot(), {
            let mut r = Robot::new(2, Point { x: 0.7, y: 0.5 });
            r.status = RobotStatus::Active;
            r
        }]
    }

    #[test]
    fn test_fire_instruction() {
        let mut robot = create_test_robot();
        let mut command_queue = VecDeque::new();
        let arena = Arena::new();
        let processor = CombatOperations::new();

        // Select turret component
        robot.vm_state.set_selected_component(2).unwrap();

        // Give robot some power
        robot.power = 1.0;

        // Execute fire instruction with power 0.5
        let fire = Instruction::Fire(Operand::Value(0.5));
        let result = processor.process(&mut robot, &[], &arena, &fire, &mut command_queue);

        // Fire should succeed
        assert!(result.is_ok());

        // Power should be reduced
        assert_eq!(robot.power, 0.5);

        // Command queue should have two commands: projectile and muzzle flash
        assert_eq!(command_queue.len(), 2);

        if let Some(ArenaCommand::SpawnProjectile(projectile)) = command_queue.pop_front() {
            assert_eq!(projectile.source_robot, robot.id);
            assert_eq!(projectile.power, 0.5);
        } else {
            panic!("Expected SpawnProjectile command");
        }

        if let Some(ArenaCommand::SpawnMuzzleFlash { .. }) = command_queue.pop_front() {
            // Muzzle flash has position and direction, but we don't check specifics
        } else {
            panic!("Expected SpawnMuzzleFlash command");
        }
    }

    #[test]
    fn test_fire_insufficient_power() {
        let mut robot = create_test_robot();
        let mut command_queue = VecDeque::new();
        let arena = Arena::new();
        let processor = CombatOperations::new();

        // Select turret component
        robot.vm_state.set_selected_component(2).unwrap();

        // Robot has no power
        robot.power = 0.0;

        // Execute fire instruction
        let fire = Instruction::Fire(Operand::Value(0.5));
        let result = processor.process(&mut robot, &[], &arena, &fire, &mut command_queue);

        // Fire should still succeed but no projectile spawned
        assert!(result.is_ok());

        // Command queue should be empty (no projectile spawned)
        assert_eq!(command_queue.len(), 0);
    }

    #[test]
    fn test_scan_instruction() {
        let mut robots = create_test_robots();
        let mut robot = robots.remove(0);
        let all_robots = robots; // This contains one other robot at (0.7, 0.5)

        let mut command_queue = VecDeque::new();
        let arena = Arena::new();
        let processor = CombatOperations::new();

        // Select turret component
        robot.vm_state.set_selected_component(2).unwrap();

        // Execute scan instruction
        let scan = Instruction::Scan;
        let result = processor.process(&mut robot, &all_robots, &arena, &scan, &mut command_queue);

        // Scan should succeed
        assert!(result.is_ok());

        // Check that the target registers were updated
        let distance = robot
            .vm_state
            .registers
            .get(Register::TargetDistance)
            .unwrap();
        let direction = robot
            .vm_state
            .registers
            .get(Register::TargetDirection)
            .unwrap();

        // Other robot is at (0.7, 0.5), we're at (0.5, 0.5), so it should be detected
        assert!(distance > 0.0); // Should have found something
        assert!(distance < 0.3); // Distance should be about 0.2
        assert_eq!(direction, 0.0); // Should be directly to the right
    }

    #[test]
    fn test_scan_no_targets() {
        let mut robot = create_test_robot();
        let all_robots = vec![]; // No other robots

        let mut command_queue = VecDeque::new();
        let arena = Arena::new();
        let processor = CombatOperations::new();

        // Select turret component
        robot.vm_state.set_selected_component(2).unwrap();

        // Execute scan instruction
        let scan = Instruction::Scan;
        let result = processor.process(&mut robot, &all_robots, &arena, &scan, &mut command_queue);

        // Scan should succeed even with no targets
        assert!(result.is_ok());

        // Check target registers - should indicate no target found
        let distance = robot
            .vm_state
            .registers
            .get(Register::TargetDistance)
            .unwrap();
        assert_eq!(distance, 0.0); // 0 distance means no target found
    }

    #[test]
    fn test_scan_by_id() {
        use crate::vm::executor::combat_ops::process_by_id;

        let mut robot = create_test_robot();
        let robot_ids = vec![1, 2];

        let mut command_queue = VecDeque::new();
        let arena = Arena::new();

        // Mock robot info provider
        let mut get_robot_info = |id: u32| -> Option<(Point, RobotStatus)> {
            if id == 2 {
                Some((Point { x: 0.7, y: 0.5 }, RobotStatus::Active))
            } else {
                None
            }
        };

        // Select turret component
        robot.vm_state.set_selected_component(2).unwrap();

        // Execute scan instruction using process_by_id
        let scan = Instruction::Scan;
        let result = process_by_id(
            &mut robot,
            &mut get_robot_info,
            &robot_ids,
            &arena,
            &scan,
            &mut command_queue,
        );

        // Scan should succeed
        assert!(result.is_ok());

        // Check that the target registers were updated correctly
        let distance = robot
            .vm_state
            .registers
            .get(Register::TargetDistance)
            .unwrap();
        let direction = robot
            .vm_state
            .registers
            .get(Register::TargetDirection)
            .unwrap();

        assert!(distance > 0.0); // Should have found robot 2
        assert!(distance < 0.3); // Distance should be about 0.2
        assert_eq!(direction, 0.0); // Should be directly to the right
    }
}
