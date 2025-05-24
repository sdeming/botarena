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
                    0..=2 => {
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
                        Ok(())
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
                        Ok(())
                    }
                    0 => Err(VMFault::NoComponentSelected),
                    _ => Err(VMFault::InvalidComponentForOp),
                }
            }
            Instruction::Drive(op) => {
                let normalized_speed = op.get_value(&robot.vm_state)?;
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
                        "Drive instruction. Normalized speed: {}",
                        normalized_speed
                    );

                    // Clamp the normalized speed to 0.0-1.0 range
                    let clamped_normalized_speed = normalized_speed.clamp(
                        -config::MAX_DRIVE_SPEED_NORMALIZED, 
                        config::MAX_DRIVE_SPEED_NORMALIZED
                    );

                    // Scale normalized speed (0.0-1.0) to actual grid units per turn (0.0-5.0)
                    let grid_units_per_turn = clamped_normalized_speed * config::MAX_DRIVE_UNITS_PER_TURN;

                    // Convert grid units per turn to coordinate units per cycle
                    let units_per_cycle = grid_units_per_turn * config::DRIVE_VELOCITY_FACTOR;

                    if clamped_normalized_speed != normalized_speed {
                        crate::debug_instructions!(
                            robot.id,
                            robot.vm_state.turn,
                            robot.vm_state.cycle,
                            "Drive speed clamped from {} to {} (normalized)",
                            normalized_speed,
                            clamped_normalized_speed
                        );
                    }

                    robot.set_drive_velocity(units_per_cycle);
                    crate::debug_instructions!(
                        robot.id,
                        robot.vm_state.turn,
                        robot.vm_state.cycle,
                        "Drive instruction set velocity to {} units per cycle ({} grid units per turn, normalized speed {})",
                        robot.drive.velocity,
                        grid_units_per_turn,
                        clamped_normalized_speed
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
    use crate::arena::Arena;
    use crate::config;
    use crate::robot::Robot;
    use crate::types::{ArenaCommand, Point};
    use crate::vm::error::VMFault;
    use crate::vm::instruction::Instruction;
    use crate::vm::operand::Operand;
    use crate::vm::registers::Register;
    use std::collections::VecDeque;

    fn create_test_robot() -> Robot {
        let arena = Arena::new();
        let center = Point {
            x: arena.width / 2.0,
            y: arena.height / 2.0,
        };
        Robot::new(1, "TestRobot".to_string(), Point { x: 0.5, y: 0.5 }, center)
    }

    fn setup() -> (Robot, Arena, VecDeque<ArenaCommand>) {
        let mut robot = create_test_robot();
        let arena = Arena::new();
        let command_queue = VecDeque::new();
        // Initialize registers for testing
        robot.vm_state.registers.set(Register::D0, 5.0).unwrap();
        robot.vm_state.registers.set(Register::D1, 10.0).unwrap();
        (robot, arena, command_queue)
    }

    #[test]
    fn test_can_process() {
        let processor = ComponentOperations::new();
        assert!(processor.can_process(&Instruction::Select(Operand::Value(1.0))));
        assert!(processor.can_process(&Instruction::Deselect));
        assert!(processor.can_process(&Instruction::Rotate(Operand::Value(90.0))));
        assert!(processor.can_process(&Instruction::Drive(Operand::Value(1.0))));

        // Should not process other instructions
        assert!(!processor.can_process(&Instruction::Add));
    }

    #[test]
    fn test_select_component() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ComponentOperations::new();
        let result = processor.process(
            &mut robot,
            &[],
            &arena,
            &Instruction::Select(Operand::Value(1.0)),
            &mut command_queue,
        );
        assert!(result.is_ok());
        assert_eq!(
            robot.vm_state.registers.get(Register::Component).unwrap(),
            1.0
        );

        let result_deselect = processor.process(
            &mut robot,
            &[],
            &arena,
            &Instruction::Deselect,
            &mut command_queue,
        );
        assert!(result_deselect.is_ok());
        assert_eq!(
            robot.vm_state.registers.get(Register::Component).unwrap(),
            0.0
        );
    }

    #[test]
    fn test_select_invalid_component() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ComponentOperations::new();
        let result = processor.process(
            &mut robot,
            &[],
            &arena,
            &Instruction::Select(Operand::Value(99.0)),
            &mut command_queue,
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), VMFault::InvalidComponentForOp);
    }

    #[test]
    fn test_drive_requires_drive_component() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ComponentOperations::new();

        // Select turret component (not drive)
        robot.vm_state.set_selected_component(2).unwrap();

        let drive = Instruction::Drive(Operand::Value(1.0));
        let result = processor.process(&mut robot, &[], &arena, &drive, &mut command_queue);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), VMFault::InvalidComponentForOp);
    }

    #[test]
    fn test_rotate_requires_drive_or_turret() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ComponentOperations::new();

        // Case 1: No component selected initially (ID 0)
        robot.vm_state.set_selected_component(0).unwrap();
        let rotate = Instruction::Rotate(Operand::Value(45.0));
        let result_none = processor.process(&mut robot, &[], &arena, &rotate, &mut command_queue);
        assert!(result_none.is_err());
        assert_eq!(
            result_none.unwrap_err(),
            VMFault::NoComponentSelected,
            "Rotate with component 0 selected should yield NoComponentSelected."
        );

        // Case 2: Select an invalid component ID (e.g., 99)
        robot.vm_state.set_selected_component(99).unwrap();
        let result_invalid =
            processor.process(&mut robot, &[], &arena, &rotate, &mut command_queue);
        assert!(result_invalid.is_err());
        assert_eq!(
            result_invalid.unwrap_err(),
            VMFault::InvalidComponentForOp,
            "Rotate with invalid component ID 99 should yield InvalidComponentForOp."
        );
    }

    #[test]
    fn test_drive_sets_velocity() {
        let mut robot = create_test_robot();
        let mut command_queue = VecDeque::new();
        let arena = Arena::new();
        let processor = ComponentOperations::new();

        // Select drive component first
        robot.vm_state.set_selected_component(1).unwrap();

        // Test with a normalized speed within the allowed range (0.0-1.0)
        let normalized_speed = 0.5; // 50% of max speed
        let expected_grid_units_per_turn = normalized_speed * config::MAX_DRIVE_UNITS_PER_TURN; // 0.5 * 5.0 = 2.5
        let expected_scaled_velocity = expected_grid_units_per_turn * config::DRIVE_VELOCITY_FACTOR;
        let drive = Instruction::Drive(Operand::Value(normalized_speed));
        let result = processor.process(&mut robot, &[], &arena, &drive, &mut command_queue);

        assert!(result.is_ok());
        assert_eq!(robot.drive.velocity, expected_scaled_velocity);

        // Test with a value exceeding the maximum (should be clamped to 1.0)
        let excessive_speed = 1.5; // 150% of max speed, should be clamped to 1.0
        let expected_max_grid_units = config::MAX_DRIVE_UNITS_PER_TURN; // 5.0 grid units per turn
        let expected_max_velocity = expected_max_grid_units * config::DRIVE_VELOCITY_FACTOR;
        let drive_excessive = Instruction::Drive(Operand::Value(excessive_speed));
        let result = processor.process(
            &mut robot,
            &[],
            &arena,
            &drive_excessive,
            &mut command_queue,
        );

        assert!(result.is_ok());
        // Verify that the value was clamped to max
        assert_eq!(robot.drive.velocity, expected_max_velocity);

        // Test with a value lower than the minimum (should be clamped to -1.0)
        let excessive_reverse_speed = -1.5; // -150% of max speed, should be clamped to -1.0
        let expected_min_grid_units = -config::MAX_DRIVE_UNITS_PER_TURN; // -5.0 grid units per turn
        let expected_min_velocity = expected_min_grid_units * config::DRIVE_VELOCITY_FACTOR;
        let reverse_drive_excessive = Instruction::Drive(Operand::Value(excessive_reverse_speed));
        let result = processor.process(
            &mut robot,
            &[],
            &arena,
            &reverse_drive_excessive,
            &mut command_queue,
        );

        assert!(result.is_ok());
        // Verify that the value was clamped to min
        assert_eq!(robot.drive.velocity, expected_min_velocity);
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
    fn test_drive_velocity_conversion() {
        let mut robot = create_test_robot();
        let mut command_queue = VecDeque::new();
        let arena = Arena::new();
        let processor = ComponentOperations::new();

        // Select drive component first
        robot.vm_state.set_selected_component(1).unwrap();

        // Test with normalized speed 0.5 (50% of max)
        let normalized_speed = 0.5;
        let expected_grid_units_per_turn = normalized_speed * config::MAX_DRIVE_UNITS_PER_TURN; // 0.5 * 5.0 = 2.5
        let expected_velocity = expected_grid_units_per_turn * config::DRIVE_VELOCITY_FACTOR;
        let drive = Instruction::Drive(Operand::Value(normalized_speed));
        let result = processor.process(&mut robot, &[], &arena, &drive, &mut command_queue);

        assert!(result.is_ok());
        assert_eq!(robot.drive.velocity, expected_velocity);

        // Test with normalized speed 1.0 (100% of max)
        let max_normalized_speed = 1.0;
        let expected_max_grid_units = config::MAX_DRIVE_UNITS_PER_TURN; // 5.0 grid units per turn
        let expected_max_velocity = expected_max_grid_units * config::DRIVE_VELOCITY_FACTOR;
        let drive_max = Instruction::Drive(Operand::Value(max_normalized_speed));
        let result = processor.process(&mut robot, &[], &arena, &drive_max, &mut command_queue);

        assert!(result.is_ok());
        assert_eq!(robot.drive.velocity, expected_max_velocity);

        // Test with excessive normalized speed (should be clamped to 1.0)
        let excessive_speed = 2.0;
        let drive_excessive = Instruction::Drive(Operand::Value(excessive_speed));
        let result = processor.process(
            &mut robot,
            &[],
            &arena,
            &drive_excessive,
            &mut command_queue,
        );

        assert!(result.is_ok());
        // Should be clamped to max (same as 1.0 normalized speed)
        assert_eq!(robot.drive.velocity, expected_max_velocity);
    }
}
