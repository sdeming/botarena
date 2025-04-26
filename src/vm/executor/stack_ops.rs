use crate::arena::Arena;
use crate::robot::Robot;
use crate::types::ArenaCommand;
use std::collections::VecDeque;

use super::processor::InstructionProcessor;
use crate::vm::error::{StackError, VMFault};
use crate::vm::instruction::Instruction;

/// Processor for stack manipulation instructions
pub struct StackOperations;

impl StackOperations {
    pub fn new() -> Self {
        StackOperations
    }
}

impl InstructionProcessor for StackOperations {
    fn can_process(&self, instruction: &Instruction) -> bool {
        matches!(
            instruction,
            Instruction::Push(_)
                | Instruction::Pop(_)
                | Instruction::PopDiscard
                | Instruction::Dup
                | Instruction::Swap
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
            Instruction::Push(op) => {
                let val = op.get_value_mut(&mut robot.vm_state)?;
                robot.vm_state.stack.push(val).map_err(|e| match e {
                    StackError::Overflow => VMFault::StackOverflow,
                    StackError::Underflow => VMFault::StackUnderflow,
                })
            }
            Instruction::Pop(reg) => {
                let val = robot.vm_state.stack.pop().map_err(|e| match e {
                    StackError::Underflow => VMFault::StackUnderflow,
                    StackError::Overflow => VMFault::StackOverflow,
                })?;
                robot
                    .vm_state
                    .registers
                    .set(*reg, val)
                    .map_err(|_| VMFault::PermissionError)
            }
            Instruction::PopDiscard => {
                robot.vm_state.stack.pop().map(|_| ()).map_err(|e| match e {
                    StackError::Underflow => VMFault::StackUnderflow,
                    StackError::Overflow => VMFault::StackOverflow,
                })
            }
            Instruction::Dup => robot.vm_state.stack.dup().map_err(|e| match e {
                StackError::Overflow => VMFault::StackOverflow,
                StackError::Underflow => VMFault::StackUnderflow,
            }),
            Instruction::Swap => robot.vm_state.stack.swap().map_err(|e| match e {
                StackError::Underflow => VMFault::StackUnderflow,
                StackError::Overflow => VMFault::StackOverflow,
            }),
            _ => Err(VMFault::InvalidInstruction),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::arena::Arena;
    use crate::robot::Robot;
    use crate::types::{ArenaCommand, Point};
    use crate::vm::error::VMFault;
    use crate::vm::executor::processor::InstructionProcessor;
    use crate::vm::executor::stack_ops::StackOperations;
    use crate::vm::instruction::Instruction;
    use crate::vm::operand::Operand;
    use crate::vm::registers::Register;
    use std::collections::VecDeque;

    fn setup() -> (Robot, Arena, VecDeque<ArenaCommand>) {
        let arena = Arena::new();
        let center = Point { x: arena.width / 2.0, y: arena.height / 2.0 };
        let mut robot = Robot::new(0, "TestRobot".to_string(), Point { x: 0.5, y: 0.5 }, center);
        let command_queue = VecDeque::new();

        // Initialize stack and registers for testing
        robot.vm_state.registers.set(Register::D0, 42.0).unwrap();

        (robot, arena, command_queue)
    }

    #[test]
    fn test_can_process() {
        let processor = StackOperations::new();

        assert!(processor.can_process(&Instruction::Push(Operand::Value(1.0))));
        assert!(processor.can_process(&Instruction::Pop(Register::D0)));
        assert!(processor.can_process(&Instruction::PopDiscard));
        assert!(processor.can_process(&Instruction::Dup));
        assert!(processor.can_process(&Instruction::Swap));

        // Should not process non-stack operations
        assert!(!processor.can_process(&Instruction::Nop));
    }

    #[test]
    fn test_push_value() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = StackOperations::new();
        let all_robots = vec![];

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Push(Operand::Value(123.0)),
            &mut command_queue,
        );

        assert!(result.is_ok());
        // The Stack doesn't have a peek method, so use pop to check the value
        assert_eq!(robot.vm_state.stack.pop().unwrap(), 123.0);
    }

    #[test]
    fn test_push_register() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = StackOperations::new();
        let all_robots = vec![];

        // Set up a register value
        robot.vm_state.registers.set(Register::D0, 42.0).unwrap();

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Push(Operand::Register(Register::D0)),
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_eq!(robot.vm_state.stack.pop().unwrap(), 42.0);
    }

    #[test]
    fn test_pop_to_register() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = StackOperations::new();
        let all_robots = vec![];

        // Push a value first
        robot.vm_state.stack.push(123.0).unwrap();

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Pop(Register::D1),
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_eq!(robot.vm_state.registers.get(Register::D1).unwrap(), 123.0);
        // Check that stack is empty by trying to pop and expecting an error
        assert!(robot.vm_state.stack.pop().is_err());
    }

    #[test]
    fn test_pop_empty_stack() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = StackOperations::new();
        let all_robots = vec![];

        // Do not push anything, so stack is empty

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Pop(Register::D1),
            &mut command_queue,
        );

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VMFault::StackUnderflow));
    }

    #[test]
    fn test_pop_discard() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = StackOperations::new();
        let all_robots = vec![];

        // Push a value first
        robot.vm_state.stack.push(123.0).unwrap();

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::PopDiscard,
            &mut command_queue,
        );

        assert!(result.is_ok());
        // Check that stack is empty by trying to pop and expecting an error
        assert!(robot.vm_state.stack.pop().is_err());
    }

    #[test]
    fn test_dup() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = StackOperations::new();
        let all_robots = vec![];

        // Push a value first
        robot.vm_state.stack.push(123.0).unwrap();

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Dup,
            &mut command_queue,
        );

        assert!(result.is_ok());

        // Pop twice to ensure we duplicated
        assert_eq!(robot.vm_state.stack.pop().unwrap(), 123.0);
        assert_eq!(robot.vm_state.stack.pop().unwrap(), 123.0);
        // Stack should now be empty
        assert!(robot.vm_state.stack.pop().is_err());
    }

    #[test]
    fn test_swap() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = StackOperations::new();
        let all_robots = vec![];

        // Push two values
        robot.vm_state.stack.push(1.0).unwrap();
        robot.vm_state.stack.push(2.0).unwrap();

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Swap,
            &mut command_queue,
        );

        assert!(result.is_ok());

        // Pop to check order
        assert_eq!(robot.vm_state.stack.pop().unwrap(), 1.0);
        assert_eq!(robot.vm_state.stack.pop().unwrap(), 2.0);
    }
}
