use crate::arena::Arena;
use crate::robot::{Robot, RobotStatus};
use crate::types::{ArenaCommand, Point};
use crate::vm::error::VMFault;
use crate::vm::registers::Register;
use std::collections::VecDeque;

use super::arithmetic_ops::ArithmeticOperations;
use super::bitwise_ops::BitwiseOperations;
use super::combat_ops::CombatOperations;
use super::component_ops::ComponentOperations;
use super::control_flow_ops::ControlFlowOperations;
use super::misc_ops::MiscellaneousOperations;
use super::processor::InstructionProcessor;
use super::register_ops::RegisterOperations;
use super::stack_ops::StackOperations;
use super::trig_ops::TrigonometricOperations;
use crate::vm::instruction::Instruction;

/// A struct that holds all instruction processors
pub struct InstructionExecutor {
    processors: Vec<Box<dyn InstructionProcessor>>,
}

impl InstructionExecutor {
    /// Create a new executor with all processors registered
    pub fn new() -> Self {
        let mut processors: Vec<Box<dyn InstructionProcessor>> = Vec::new();

        // Add all processors here
        processors.push(Box::new(StackOperations::new()));
        processors.push(Box::new(RegisterOperations::new()));
        processors.push(Box::new(ArithmeticOperations::new()));
        processors.push(Box::new(TrigonometricOperations::new()));
        processors.push(Box::new(BitwiseOperations::new()));
        processors.push(Box::new(ControlFlowOperations::new()));
        processors.push(Box::new(ComponentOperations::new()));
        processors.push(Box::new(CombatOperations::new()));
        processors.push(Box::new(MiscellaneousOperations::new()));

        InstructionExecutor { processors }
    }

    /// Execute a single instruction, delegating to the appropriate processor
    pub fn execute_instruction(
        &self,
        robot: &mut Robot,
        all_robots: &[Robot],
        arena: &Arena,
        instr: &Instruction,
        command_queue: &mut VecDeque<ArenaCommand>,
    ) -> Result<(), VMFault> {
        // Check for processor that can handle this instruction
        for processor in &self.processors {
            if processor.can_process(instr) {
                let result = processor.process(robot, all_robots, arena, instr, command_queue);

                // Handle fault register
                if result.is_ok() && robot.vm_state.fault.is_some() {
                    robot.vm_state.fault = None;
                    robot
                        .vm_state
                        .registers
                        .set_internal(Register::Fault, 0.0)
                        .unwrap();
                } else if let Err(ref fault) = result {
                    robot.vm_state.set_fault(*fault);
                }

                return result;
            }
        }

        // No processor found to handle this instruction
        Err(VMFault::InvalidInstruction)
    }

    /// Execute an instruction by ID using the appropriate processor
    pub fn execute_instruction_by_id<F>(
        &self,
        robot: &mut Robot,
        get_robot_info: &mut F,
        robot_ids: &[u32],
        arena: &Arena,
        instr: &Instruction,
        command_queue: &mut VecDeque<ArenaCommand>,
    ) -> Result<(), VMFault>
    where
        F: FnMut(u32) -> Option<(Point, RobotStatus)>,
    {
        // Special case for Scan which needs access to robot IDs
        if matches!(instr, Instruction::Scan) {
            return super::combat_ops::process_by_id(
                robot,
                get_robot_info,
                robot_ids,
                arena,
                instr,
                command_queue,
            );
        }

        // For all other instructions, delegate to normal execute_instruction
        self.execute_instruction(robot, &[], arena, instr, command_queue)
    }
}

#[cfg(test)]
mod tests {
    use crate::arena::Arena;
    use crate::robot::Robot;
    use crate::types::Point;
    use crate::vm::executor::instruction_executor::InstructionExecutor;
    use crate::vm::instruction::Instruction;
    use crate::vm::operand::Operand;
    use crate::vm::registers::Register;
    use std::collections::VecDeque;

    #[test]
    fn test_stack_operations_delegation() {
        let mut robot = Robot::new(0, Point { x: 0.5, y: 0.5 });
        let arena = Arena::new();
        let mut command_queue = VecDeque::new();
        let all_robots = vec![];

        let executor = InstructionExecutor::new();

        // Test Push instruction
        let result = executor.execute_instruction(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Push(Operand::Value(42.0)),
            &mut command_queue,
        );

        assert!(result.is_ok());

        // Test Pop instruction
        let result = executor.execute_instruction(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Pop(Register::D0),
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_eq!(robot.vm_state.registers.get(Register::D0).unwrap(), 42.0);
    }

    #[test]
    fn test_register_operations_delegation() {
        let mut robot = Robot::new(0, Point { x: 0.5, y: 0.5 });
        let arena = Arena::new();
        let mut command_queue = VecDeque::new();
        let all_robots = vec![];

        let executor = InstructionExecutor::new();

        // Test Mov instruction
        let result = executor.execute_instruction(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Mov(Register::D1, Operand::Value(55.0)),
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_eq!(robot.vm_state.registers.get(Register::D1).unwrap(), 55.0);

        // Test Cmp instruction
        let result = executor.execute_instruction(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Cmp(Operand::Value(10.0), Operand::Value(5.0)),
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_eq!(robot.vm_state.registers.get(Register::Result).unwrap(), 5.0);
    }

    #[test]
    fn test_arithmetic_operations_delegation() {
        let mut robot = Robot::new(0, Point { x: 0.5, y: 0.5 });
        let arena = Arena::new();
        let mut command_queue = VecDeque::new();
        let all_robots = vec![];

        let executor = InstructionExecutor::new();

        // Test stack-based arithmetic
        // Push two values
        robot.vm_state.stack.push(10.0).unwrap();
        robot.vm_state.stack.push(3.0).unwrap();

        // Test Add
        let result = executor.execute_instruction(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Add,
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_eq!(robot.vm_state.stack.pop().unwrap(), 13.0);

        // Test register-based arithmetic
        // Test MulOp
        let result = executor.execute_instruction(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::MulOp(Operand::Value(4.0), Operand::Value(5.0)),
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_eq!(
            robot.vm_state.registers.get(Register::Result).unwrap(),
            20.0
        );

        // Test PowOp
        let result = executor.execute_instruction(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::PowOp(Operand::Value(2.0), Operand::Value(3.0)),
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_eq!(robot.vm_state.registers.get(Register::Result).unwrap(), 8.0);
    }

    #[test]
    fn test_fallback_to_original_implementation() {
        let mut robot = Robot::new(0, Point { x: 0.5, y: 0.5 });
        let arena = Arena::new();
        let mut command_queue = VecDeque::new();
        let all_robots = vec![];

        let executor = InstructionExecutor::new();

        // Test Nop instruction (not yet implemented in a processor)
        let result = executor.execute_instruction(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Nop,
            &mut command_queue,
        );

        // Should succeed by falling back to the original implementation
        assert!(result.is_ok());
    }
}
