use crate::arena::Arena;
use crate::robot::Robot;
use crate::types::ArenaCommand;
use std::collections::VecDeque;

use super::processor::InstructionProcessor;
use crate::vm::error::VMFault;
use crate::vm::instruction::Instruction;

/// Processor for register manipulation instructions
pub struct RegisterOperations;

impl RegisterOperations {
    pub fn new() -> Self {
        RegisterOperations
    }
}

impl InstructionProcessor for RegisterOperations {
    fn can_process(&self, instruction: &Instruction) -> bool {
        matches!(
            instruction,
            Instruction::Mov(_, _)
                | Instruction::Lod(_)
                | Instruction::Sto(_)
                | Instruction::Cmp(_, _)
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
        match instruction {
            Instruction::Mov(reg, op) => {
                let val = op.get_value(&robot.vm_state)?;
                let result = robot
                    .vm_state
                    .registers
                    .set(*reg, val)
                    .map_err(|_| VMFault::PermissionError);

                // Special handling for @d7 register
                if let Ok(()) = result {
                    if let crate::vm::registers::Register::D7 = reg {
                        crate::debug_instructions!(
                            robot.id,
                            robot.vm_state.turn,
                            robot.vm_state.cycle,
                            "Mov: Setting @d7 to {:.1}",
                            val
                        );
                    }
                }

                result
            }
            Instruction::Lod(reg) => {
                // Load from memory at @index to register
                let value = robot.vm_state.load_memory_at_index()?;
                robot
                    .vm_state
                    .registers
                    .set(*reg, value)
                    .map_err(|_| VMFault::PermissionError)?;

                crate::debug_instructions!(
                    robot.id,
                    robot.vm_state.turn,
                    robot.vm_state.cycle,
                    "Lod: Loaded {:.1} from memory to {:?}, index auto-incremented",
                    value,
                    reg
                );
                Ok(())
            }
            Instruction::Sto(op) => {
                // Store operand to memory at @index
                let value = op.get_value(&robot.vm_state)?;
                robot.vm_state.store_memory_at_index(value)?;

                crate::debug_instructions!(
                    robot.id,
                    robot.vm_state.turn,
                    robot.vm_state.cycle,
                    "Sto: Stored {:.1} to memory, index auto-incremented",
                    value
                );
                Ok(())
            }
            Instruction::Cmp(left, right) => {
                // Use immutable access for reading registers
                let left_val = left.get_value(&robot.vm_state)?;
                let right_val = right.get_value(&robot.vm_state)?;
                let result_val = left_val - right_val;
                robot
                    .vm_state
                    .registers
                    .set(crate::vm::registers::Register::Result, result_val)
                    .map_err(|_| VMFault::PermissionError)
            }
            _ => Err(VMFault::InvalidInstruction),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::Arena;
    use crate::robot::{Robot, RobotStatus};
    use crate::types::{Point, ArenaCommand};
    use crate::vm::error::VMFault;
    use crate::vm::executor::{processor::InstructionProcessor, InstructionExecutor};
    use crate::vm::instruction::Instruction;
    use crate::vm::operand::Operand;
    use crate::vm::registers::Register;
    use crate::vm::state::VMState;
    use std::collections::VecDeque;

    fn execute_instruction(
        robot: &mut Robot,
        arena: &Arena,
        instruction: &Instruction,
        command_queue: &mut VecDeque<ArenaCommand>,
    ) -> Result<(), VMFault> {
        let executor = InstructionExecutor::new();
        let all_robots = vec![]; // Use empty vec for register ops tests
        executor.execute_instruction(robot, &all_robots, arena, instruction, command_queue)
    }

    fn setup_vm_state() -> (Robot, Arena, VecDeque<ArenaCommand>) {
        let arena = Arena::new();
        let center = Point { x: arena.width / 2.0, y: arena.height / 2.0 };
        let mut robot = Robot::new(0, "TestRobot".to_string(), Point { x: 0.5, y: 0.5 }, center);
        let command_queue = VecDeque::new();

        // Initialize registers for testing
        robot.vm_state.registers.set(Register::D0, 5.0).unwrap();
        robot.vm_state.registers.set(Register::D1, 10.0).unwrap();
        robot.vm_state.registers.set(Register::Result, 1.0).unwrap();
        robot.vm_state.registers.set(Register::Index, 0.0).unwrap();

        // Initialize memory for Lod/Sto tests
        robot.vm_state.memory[0] = 5.0;
        robot.vm_state.memory[1] = 10.0;
        robot.vm_state.memory[2] = 15.0;

        (robot, arena, command_queue)
    }

    fn setup_readonly_test() -> (Robot, Arena, VecDeque<ArenaCommand>) {
        let arena = Arena::new();
        let center = Point { x: arena.width / 2.0, y: arena.height / 2.0 };
        let robot = Robot::new(1, "TestRobot".to_string(), Point { x: 0.0, y: 0.0 }, center);
        let command_queue = VecDeque::new();
        (robot, arena, command_queue)
    }

    fn setup_index_test() -> (Robot, Arena, VecDeque<ArenaCommand>) {
        let arena = Arena::new();
        let center = Point { x: arena.width / 2.0, y: arena.height / 2.0 };
        let mut robot = Robot::new(1, "TestRobot".to_string(), Point { x: 0.0, y: 0.0 }, center);
        let command_queue = VecDeque::new();
        (robot, arena, command_queue)
    }

    #[test]
    fn test_can_process() {
        let processor = RegisterOperations::new();

        assert!(processor.can_process(&Instruction::Mov(Register::D0, Operand::Value(1.0))));
        assert!(processor.can_process(&Instruction::Lod(Register::D0)));
        assert!(processor.can_process(&Instruction::Sto(Operand::Value(1.0))));
        assert!(processor.can_process(&Instruction::Cmp(Operand::Value(1.0), Operand::Value(2.0))));

        // Should not process non-register operations
        assert!(!processor.can_process(&Instruction::Push(Operand::Value(1.0))));
        assert!(!processor.can_process(&Instruction::Nop));
    }

    #[test]
    fn test_mov_value_to_register() {
        let (mut robot, arena, mut command_queue) = setup_vm_state();
        let processor = RegisterOperations::new();
        let all_robots = vec![];

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Mov(Register::D2, Operand::Value(123.0)),
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_eq!(robot.vm_state.registers.get(Register::D2).unwrap(), 123.0);
    }

    #[test]
    fn test_mov_register_to_register() {
        let (mut robot, arena, mut command_queue) = setup_vm_state();
        let processor = RegisterOperations::new();
        let all_robots = vec![];

        // Set up a source register value
        robot.vm_state.registers.set(Register::D0, 42.0).unwrap();

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Mov(Register::D2, Operand::Register(Register::D0)),
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_eq!(robot.vm_state.registers.get(Register::D2).unwrap(), 42.0);
    }

    #[test]
    fn test_mov_to_readonly_register() {
        let (mut robot, arena, mut command_queue) = setup_vm_state();
        let processor = RegisterOperations::new();
        let all_robots = vec![];

        // Try to write to a read-only register
        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Mov(Register::Turn, Operand::Value(123.0)),
            &mut command_queue,
        );

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VMFault::PermissionError));
    }

    #[test]
    fn test_lod_instruction() {
        let (mut robot, arena, mut command_queue) = setup_vm_state();
        
        let result_lod1 = execute_instruction(&mut robot, &arena, &Instruction::Lod(Register::D2), &mut command_queue);
        assert!(result_lod1.is_ok(), "First Lod failed");

        assert_eq!(robot.vm_state.registers.get(Register::D2).unwrap(), 5.0);
        assert_eq!(robot.vm_state.registers.get(Register::Index).unwrap(), 1.0);

        let result_lod2 = execute_instruction(&mut robot, &arena, &Instruction::Lod(Register::D3), &mut command_queue);
        assert!(result_lod2.is_ok(), "Second Lod failed");
        
        assert_eq!(robot.vm_state.registers.get(Register::D3).unwrap(), 10.0);
        assert_eq!(robot.vm_state.registers.get(Register::Index).unwrap(), 2.0);
    }

    #[test]
    fn test_sto_instruction() {
        let (mut robot, arena, mut command_queue) = setup_vm_state();
        let processor = RegisterOperations::new();
        let all_robots = vec![];

        // Set index to memory position 5 (unused)
        robot.vm_state.registers.set(Register::Index, 5.0).unwrap();

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Sto(Operand::Value(99.0)),
            &mut command_queue,
        );

        assert!(result.is_ok());

        // Index should auto-increment to 6
        assert_eq!(robot.vm_state.registers.get(Register::Index).unwrap(), 6.0);

        // Set index back to 5 to verify the stored value
        robot.vm_state.registers.set(Register::Index, 5.0).unwrap();

        // Load the value we just stored
        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Lod(Register::D4),
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_eq!(robot.vm_state.registers.get(Register::D4).unwrap(), 99.0);
    }

    #[test]
    fn test_cmp_instruction_equal() {
        let (mut robot, arena, mut command_queue) = setup_vm_state();
        let processor = RegisterOperations::new();
        let all_robots = vec![];

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Cmp(Operand::Value(10.0), Operand::Value(10.0)),
            &mut command_queue,
        );

        assert!(result.is_ok());
        // Equal operands should result in 0.0 in the result register
        assert_eq!(robot.vm_state.registers.get(Register::Result).unwrap(), 0.0);
    }

    #[test]
    fn test_cmp_instruction_greater() {
        let (mut robot, arena, mut command_queue) = setup_vm_state();
        let processor = RegisterOperations::new();
        let all_robots = vec![];

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Cmp(Operand::Value(20.0), Operand::Value(10.0)),
            &mut command_queue,
        );

        assert!(result.is_ok());
        // 20.0 - 10.0 = 10.0 in the result register
        assert_eq!(
            robot.vm_state.registers.get(Register::Result).unwrap(),
            10.0
        );
    }

    #[test]
    fn test_cmp_instruction_less() {
        let (mut robot, arena, mut command_queue) = setup_vm_state();
        let processor = RegisterOperations::new();
        let all_robots = vec![];

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Cmp(Operand::Value(5.0), Operand::Value(10.0)),
            &mut command_queue,
        );

        assert!(result.is_ok());
        // 5.0 - 10.0 = -5.0 in the result register
        assert_eq!(
            robot.vm_state.registers.get(Register::Result).unwrap(),
            -5.0
        );
    }

    #[test]
    fn test_memory_operations_integration() {
        let mut queue = VecDeque::new();
        let arena = Arena::new(); // Define arena first
        let center = Point { x: arena.width / 2.0, y: arena.height / 2.0 }; // Calculate center
        let mut robot = Robot::new(1, "TestRobot".to_string(), Point { x: 0.0, y: 0.0 }, center); // Pass center
        let empty_robots = Vec::new();
        let executor = InstructionExecutor::new();

        // Set @index to 0
        let mov_index = Instruction::Mov(Register::Index, Operand::Value(0.0));
        assert!(
            executor
                .execute_instruction(&mut robot, &empty_robots, &arena, &mov_index, &mut queue)
                .is_ok()
        );

        // Store a value in memory
        let store_val = Instruction::Sto(Operand::Value(42.0));
        assert!(
            executor
                .execute_instruction(&mut robot, &empty_robots, &arena, &store_val, &mut queue)
                .is_ok()
        );

        // Verify @index auto-incremented
        assert_eq!(robot.vm_state.registers.get(Register::Index).unwrap(), 1.0);

        // Store another value in memory
        let store_val2 = Instruction::Sto(Operand::Value(43.0));
        assert!(
            executor
                .execute_instruction(&mut robot, &empty_robots, &arena, &store_val2, &mut queue)
                .is_ok()
        );

        // @index should now be 2
        assert_eq!(robot.vm_state.registers.get(Register::Index).unwrap(), 2.0);

        // Reset @index to 0
        let mov_index2 = Instruction::Mov(Register::Index, Operand::Value(0.0));
        assert!(
            executor
                .execute_instruction(&mut robot, &empty_robots, &arena, &mov_index2, &mut queue)
                .is_ok()
        );

        // Load the first value
        let load_val = Instruction::Lod(Register::D0);
        assert!(
            executor
                .execute_instruction(&mut robot, &empty_robots, &arena, &load_val, &mut queue)
                .is_ok()
        );

        // Check the value was loaded correctly
        assert_eq!(robot.vm_state.registers.get(Register::D0).unwrap(), 42.0);

        // @index should be 1
        assert_eq!(robot.vm_state.registers.get(Register::Index).unwrap(), 1.0);

        // Load the second value
        let load_val2 = Instruction::Lod(Register::D1);
        assert!(
            executor
                .execute_instruction(&mut robot, &empty_robots, &arena, &load_val2, &mut queue)
                .is_ok()
        );

        // Check the value was loaded correctly
        assert_eq!(robot.vm_state.registers.get(Register::D1).unwrap(), 43.0);

        // @index should be 2
        assert_eq!(robot.vm_state.registers.get(Register::Index).unwrap(), 2.0);

        // Test out of bounds access
        let mov_index3 = Instruction::Mov(Register::Index, Operand::Value(9999.0)); // Beyond memory size
        assert!(
            executor
                .execute_instruction(&mut robot, &empty_robots, &arena, &mov_index3, &mut queue)
                .is_ok()
        );

        // Load should fail with VMFault
        let load_val3 = Instruction::Lod(Register::D2);
        assert!(
            executor
                .execute_instruction(&mut robot, &empty_robots, &arena, &load_val3, &mut queue)
                .is_err()
        );
    }
}
