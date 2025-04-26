use crate::arena::Arena;
use crate::config;
use crate::robot::Robot;
use crate::types::ArenaCommand;
use crate::vm::error::VMFault;
use crate::vm::registers::Register;
use std::collections::VecDeque;

use super::processor::InstructionProcessor;
use crate::vm::instruction::Instruction;

/// Processor for robot component operations
pub struct ComponentOperations;

impl ComponentOperations {
    pub fn new() -> Self {
        ComponentOperations
    }
}

impl InstructionProcessor for ComponentOperations {
    fn can_process(&self, instruction: &Instruction) -> bool {
        matches!(
            instruction,
            Instruction::Select(_)
                | Instruction::Deselect
                | Instruction::Rotate(_)
                | Instruction::Drive(_)
        )
    }

    fn process(
        &self,
        robot: &mut Robot,
        _all_robots: &[Robot],
        _arena: &Arena,
        instruction: &Instruction,
        _command_queue: &mut VecDeque<ArenaCommand>,
    ) -> Result<(), VMFault> {
        let selected_component = robot
            .vm_state
            .registers
            .get(Register::Component)
            .unwrap_or(0.0) as u8;

        match instruction {
            Instruction::Select(op) => {
                let component_id = op.get_value_mut(&mut robot.vm_state)? as u8;
                crate::debug_instructions!(
                    robot.id,
                    robot.vm_state.turn,
                    robot.vm_state.cycle,
                    "selecting component {} (current @comp={})",
                    component_id,
                    selected_component
                );
                match component_id {
                    0 | 1 | 2 => {
                        let res = robot.vm_state.set_selected_component(component_id);
                        crate::debug_instructions!(
                            robot.id,
                            robot.vm_state.turn,
                            robot.vm_state.cycle,
                            "set component result: {:?}",
                            res
                        );
                        res
                    }
                    .map_err(|_| VMFault::InvalidComponentForOp),
                    _ => Err(VMFault::InvalidComponentForOp),
                }
            }
            Instruction::Deselect => robot
                .vm_state
                .set_selected_component(0)
                .map_err(|_| VMFault::PermissionError),
            Instruction::Rotate(op) => {
                let angle = op.get_value(&robot.vm_state)?;
                let component_val = robot
                    .vm_state
                    .registers
                    .get(Register::Component)
                    .map_err(|_| VMFault::InvalidRegister)?
                    as u8;

                crate::debug_instructions!(
                    robot.id,
                    robot.vm_state.turn,
                    robot.vm_state.cycle,
                    "Rotate: angle = {:.2}",
                    angle
                );

                crate::debug_instructions!(
                    robot.id,
                    robot.vm_state.turn,
                    robot.vm_state.cycle,
                    "Executing rotate {} with component={}, current @comp={}",
                    angle,
                    component_val,
                    component_val
                );

                // Apply rotation based on selected component
                match component_val {
                    1 => {
                        // Drive
                        crate::debug_instructions!(
                            robot.id,
                            robot.vm_state.turn,
                            robot.vm_state.cycle,
                            "Requesting drive rotation: {} (current dir={})",
                            angle,
                            robot.drive.direction
                        );
                        robot.request_drive_rotation(angle);
                    }
                    2 => {
                        // Turret
                        crate::debug_instructions!(
                            robot.id,
                            robot.vm_state.turn,
                            robot.vm_state.cycle,
                            "Requesting turret rotation: {} (current dir={})",
                            angle,
                            robot.turret.direction
                        );
                        robot.request_turret_rotation(angle);
                    }
                    _ => return Err(VMFault::NoComponentSelected),
                }

                Ok(())
            }
            Instruction::Drive(op) => {
                let val = op.get_value(&robot.vm_state)?;
                let selected_component = robot
                    .vm_state
                    .registers
                    .get(Register::Component)
                    .unwrap_or(0.0) as u8;
                if selected_component == 1 {
                    // Drive component required
                    crate::debug_instructions!(
                        robot.id,
                        robot.vm_state.turn,
                        robot.vm_state.cycle,
                        "Drive instruction. Value: {}",
                        val
                    );

                    // When user inputs drive 1.0, we want the robot to move 1.0 GRID unit per TURN
                    // A grid unit is config::UNIT_SIZE coordinate units (0.05)
                    // So we convert grid units to coordinate units per cycle:
                    // grid_units * UNIT_SIZE / CYCLES_PER_TURN = coordinate_units_per_cycle
                    let units_per_cycle = val * config::DRIVE_VELOCITY_FACTOR;
                    
                    // Clamp to a maximum (let's say max is ±5 grid units per turn, or ±0.25 coordinate units)
                    let max_units_per_cycle = config::MAX_DRIVE_UNITS_PER_TURN * config::DRIVE_VELOCITY_FACTOR;
                    let clamped_velocity = units_per_cycle.clamp(-max_units_per_cycle, max_units_per_cycle);
                    
                    if clamped_velocity != units_per_cycle {
                        crate::debug_instructions!(
                            robot.id,
                            robot.vm_state.turn,
                            robot.vm_state.cycle,
                            "Drive velocity clamped from {} to {} coordinate units per cycle",
                            units_per_cycle,
                            clamped_velocity
                        );
                    }

                    robot.set_drive_velocity(clamped_velocity);
                    crate::debug_instructions!(
                        robot.id,
                        robot.vm_state.turn,
                        robot.vm_state.cycle,
                        "Drive instruction set velocity to {} units per cycle ({} units per turn)",
                        robot.drive.velocity,
                        robot.drive.velocity * config::CYCLES_PER_TURN as f64 / config::UNIT_SIZE
                    );

                    // Update the velocity register to reflect the new target velocity
                    robot
                        .vm_state
                        .registers
                        .set_internal(Register::DriveVelocity, robot.drive.velocity)
                        .unwrap();

                    Ok(())
                } else {
                    crate::debug_instructions!(
                        robot.id,
                        robot.vm_state.turn,
                        robot.vm_state.cycle,
                        "Drive instruction FAILED - Invalid component (selected: {})",
                        selected_component
                    );
                    Err(VMFault::InvalidComponentForOp)
                }
            }
            _ => Err(VMFault::InvalidInstruction),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::robot::{Robot, RobotStatus};
    use crate::types::{Point, ArenaCommand};
    use crate::arena::Arena;
    use crate::vm::error::VMFault;
    use crate::vm::operand::Operand;
    use crate::vm::instruction::Instruction;
    use std::collections::VecDeque;

    fn create_test_robot() -> Robot {
        let mut robot = Robot::new(1, "TestRobot".to_string(), Point { x: 0.5, y: 0.5 });
        robot.status = RobotStatus::Active;
        robot
    }

    #[test]
    fn test_select_component() {
        let mut robot = create_test_robot();
        let mut command_queue = VecDeque::new();
        let arena = Arena::new();
        let processor = ComponentOperations::new();

        // Test selecting drive component (1)
        let select_drive = Instruction::Select(Operand::Value(1.0));
        let result = processor.process(&mut robot, &[], &arena, &select_drive, &mut command_queue);
        assert!(result.is_ok());
        assert_eq!(
            robot.vm_state.registers.get(Register::Component).unwrap(),
            1.0
        );

        // Test selecting turret component (2)
        let select_turret = Instruction::Select(Operand::Value(2.0));
        let result = processor.process(&mut robot, &[], &arena, &select_turret, &mut command_queue);
        assert!(result.is_ok());
        assert_eq!(
            robot.vm_state.registers.get(Register::Component).unwrap(),
            2.0
        );

        // Test selecting invalid component (3)
        let select_invalid = Instruction::Select(Operand::Value(3.0));
        let result =
            processor.process(&mut robot, &[], &arena, &select_invalid, &mut command_queue);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), VMFault::InvalidComponentForOp);
    }

    #[test]
    fn test_deselect_component() {
        let mut robot = create_test_robot();
        let mut command_queue = VecDeque::new();
        let arena = Arena::new();
        let processor = ComponentOperations::new();

        // First select a component
        robot.vm_state.set_selected_component(1).unwrap();
        assert_eq!(
            robot.vm_state.registers.get(Register::Component).unwrap(),
            1.0
        );

        // Test deselect
        let deselect = Instruction::Deselect;
        let result = processor.process(&mut robot, &[], &arena, &deselect, &mut command_queue);
        assert!(result.is_ok());
        assert_eq!(
            robot.vm_state.registers.get(Register::Component).unwrap(),
            0.0
        );
    }

    #[test]
    fn test_rotate_drive() {
        let mut robot = create_test_robot();
        let mut command_queue = VecDeque::new();
        let arena = Arena::new();
        let processor = ComponentOperations::new();

        // Select drive component
        robot.vm_state.set_selected_component(1).unwrap();

        // Initial direction
        let initial_direction = 0.0;
        robot.drive.direction = initial_direction;

        // Test rotating drive by 45 degrees
        let rotate_angle = 45.0;
        let rotate = Instruction::Rotate(Operand::Value(rotate_angle));
        let result = processor.process(&mut robot, &[], &arena, &rotate, &mut command_queue);

        assert!(result.is_ok());
        assert_eq!(robot.drive.pending_rotation, rotate_angle);
    }

    #[test]
    fn test_rotate_turret() {
        let mut robot = create_test_robot();
        let mut command_queue = VecDeque::new();
        let arena = Arena::new();
        let processor = ComponentOperations::new();

        // Select turret component
        robot.vm_state.set_selected_component(2).unwrap();

        // Initial direction
        let initial_direction = 0.0;
        robot.turret.direction = initial_direction;

        // Test rotating turret by 90 degrees
        let rotate_angle = 90.0;
        let rotate = Instruction::Rotate(Operand::Value(rotate_angle));
        let result = processor.process(&mut robot, &[], &arena, &rotate, &mut command_queue);

        assert!(result.is_ok());
        assert_eq!(robot.turret.pending_rotation, rotate_angle);
    }

    #[test]
    fn test_rotate_no_component() {
        let mut robot = create_test_robot();
        let mut command_queue = VecDeque::new();
        let arena = Arena::new();
        let processor = ComponentOperations::new();

        // No component selected
        robot.vm_state.set_selected_component(0).unwrap();

        // Try to rotate
        let rotate = Instruction::Rotate(Operand::Value(45.0));
        let result = processor.process(&mut robot, &[], &arena, &rotate, &mut command_queue);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), VMFault::NoComponentSelected);
    }

    #[test]
    fn test_drive_instruction() {
        let mut robot = create_test_robot();
        let mut command_queue = VecDeque::new();
        let arena = Arena::new();
        let processor = ComponentOperations::new();

        // Select drive component first
        robot.vm_state.set_selected_component(1).unwrap();

        // Test with a value within the allowed range
        let drive_velocity = 0.5;
        let expected_scaled_velocity = drive_velocity * config::DRIVE_VELOCITY_FACTOR;
        let drive = Instruction::Drive(Operand::Value(drive_velocity));
        let result = processor.process(&mut robot, &[], &arena, &drive, &mut command_queue);

        assert!(result.is_ok());
        assert_eq!(robot.drive.velocity, expected_scaled_velocity);

        // Test with a value exceeding the maximum
        let excessive_velocity = config::MAX_DRIVE_UNITS_PER_TURN + 1.0;
        let expected_max = config::MAX_DRIVE_UNITS_PER_TURN * config::DRIVE_VELOCITY_FACTOR; // This is now 5 * UNIT_SIZE / CYCLES_PER_TURN
        let drive_excessive = Instruction::Drive(Operand::Value(excessive_velocity));
        let result = processor.process(&mut robot, &[], &arena, &drive_excessive, &mut command_queue);

        assert!(result.is_ok());
        // Verify that the value was clamped to max
        assert_eq!(robot.drive.velocity, expected_max);

        // Test with a value lower than the minimum
        let excessive_reverse_velocity = -1.0 * (config::MAX_DRIVE_UNITS_PER_TURN + 1.0);
        let expected_min = -1.0 * (config::MAX_DRIVE_UNITS_PER_TURN * config::DRIVE_VELOCITY_FACTOR);
        let reverse_drive_excessive = Instruction::Drive(Operand::Value(excessive_reverse_velocity));
        let result = processor.process(&mut robot, &[], &arena, &reverse_drive_excessive, &mut command_queue);

        assert!(result.is_ok());
        // Verify that the value was clamped to max
        assert_eq!(robot.drive.velocity, expected_min);
    }

    #[test]
    fn test_drive_wrong_component() {
        let mut robot = create_test_robot();
        let mut command_queue = VecDeque::new();
        let arena = Arena::new();
        let processor = ComponentOperations::new();

        // Select turret component
        robot.vm_state.set_selected_component(2).unwrap();

        // Try to drive with turret selected
        let drive = Instruction::Drive(Operand::Value(0.5));
        let result = processor.process(&mut robot, &[], &arena, &drive, &mut command_queue);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), VMFault::InvalidComponentForOp);
    }

    fn setup() -> (Robot, Arena, VecDeque<ArenaCommand>) {
        // Robot with default components
        let robot = Robot::new(1, "TestRobot".to_string(), Point { x: 0.5, y: 0.5 });
        let arena = Arena::new();
        let command_queue = VecDeque::new();
        (robot, arena, command_queue)
    }
}
