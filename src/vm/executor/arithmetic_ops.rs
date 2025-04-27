use crate::arena::Arena;
use crate::robot::Robot;
use crate::types::ArenaCommand;
use std::collections::VecDeque;

use super::processor::InstructionProcessor;
use crate::vm::error::VMFault;
use crate::vm::instruction::Instruction;
use crate::vm::registers::Register;

/// Processor for arithmetic operations
pub struct ArithmeticOperations;

impl ArithmeticOperations {
    pub fn new() -> Self {
        ArithmeticOperations
    }
}

impl InstructionProcessor for ArithmeticOperations {
    fn can_process(&self, instruction: &Instruction) -> bool {
        matches!(
            instruction,
            // Stack-based operations
            Instruction::Add |
            Instruction::Sub |
            Instruction::Mul |
            Instruction::Div |
            Instruction::Mod |
            Instruction::Divmod |
            Instruction::Pow |
            Instruction::Sqrt |
            Instruction::Log |
            // Register-based operations
            Instruction::AddOp(_, _) |
            Instruction::SubOp(_, _) |
            Instruction::MulOp(_, _) |
            Instruction::DivOp(_, _) |
            Instruction::ModOp(_, _) |
            Instruction::PowOp(_, _) |
            Instruction::SqrtOp(_) |
            Instruction::LogOp(_)
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
            // Stack-based arithmetic operations
            Instruction::Add => {
                let b = robot
                    .vm_state
                    .stack
                    .pop()
                    .map_err(|_| VMFault::StackUnderflow)?;
                let a = robot
                    .vm_state
                    .stack
                    .pop()
                    .map_err(|_| VMFault::StackUnderflow)?;
                robot
                    .vm_state
                    .stack
                    .push(a + b)
                    .map_err(|_| VMFault::StackOverflow)
            }
            Instruction::Sub => {
                let b = robot
                    .vm_state
                    .stack
                    .pop()
                    .map_err(|_| VMFault::StackUnderflow)?;
                let a = robot
                    .vm_state
                    .stack
                    .pop()
                    .map_err(|_| VMFault::StackUnderflow)?;
                robot
                    .vm_state
                    .stack
                    .push(a - b)
                    .map_err(|_| VMFault::StackOverflow)
            }
            Instruction::Mul => {
                let b = robot
                    .vm_state
                    .stack
                    .pop()
                    .map_err(|_| VMFault::StackUnderflow)?;
                let a = robot
                    .vm_state
                    .stack
                    .pop()
                    .map_err(|_| VMFault::StackUnderflow)?;
                robot
                    .vm_state
                    .stack
                    .push(a * b)
                    .map_err(|_| VMFault::StackOverflow)
            }
            Instruction::Div => {
                let b = robot
                    .vm_state
                    .stack
                    .pop()
                    .map_err(|_| VMFault::StackUnderflow)?;
                if b == 0.0 {
                    return Err(VMFault::DivisionByZero);
                }
                let a = robot
                    .vm_state
                    .stack
                    .pop()
                    .map_err(|_| VMFault::StackUnderflow)?;
                robot
                    .vm_state
                    .stack
                    .push(a / b)
                    .map_err(|_| VMFault::StackOverflow)
            }
            Instruction::Mod => {
                let b = robot
                    .vm_state
                    .stack
                    .pop()
                    .map_err(|_| VMFault::StackUnderflow)?;
                if b == 0.0 {
                    return Err(VMFault::DivisionByZero);
                }
                let a = robot
                    .vm_state
                    .stack
                    .pop()
                    .map_err(|_| VMFault::StackUnderflow)?;
                robot
                    .vm_state
                    .stack
                    .push(a % b)
                    .map_err(|_| VMFault::StackOverflow)
            }
            Instruction::Divmod => {
                let b = robot
                    .vm_state
                    .stack
                    .pop()
                    .map_err(|_| VMFault::StackUnderflow)?;
                let a = robot
                    .vm_state
                    .stack
                    .pop()
                    .map_err(|_| VMFault::StackUnderflow)?;

                if b == 0.0 {
                    return Err(VMFault::DivisionByZero);
                }

                // Calculate quotient and remainder
                let quotient = (a / b).floor(); // Integer division (floor)
                let remainder = a % b; // Modulo operation

                // Push remainder first, then quotient (so popping gives quotient first)
                robot
                    .vm_state
                    .stack
                    .push(remainder)
                    .map_err(|_| VMFault::StackOverflow)?;
                robot
                    .vm_state
                    .stack
                    .push(quotient)
                    .map_err(|_| VMFault::StackOverflow)
            }
            Instruction::Pow => {
                let exponent = robot
                    .vm_state
                    .stack
                    .pop()
                    .map_err(|_| VMFault::StackUnderflow)?;
                let base = robot
                    .vm_state
                    .stack
                    .pop()
                    .map_err(|_| VMFault::StackUnderflow)?;
                robot
                    .vm_state
                    .stack
                    .push(base.powf(exponent))
                    .map_err(|_| VMFault::StackOverflow)
            }
            Instruction::Sqrt => {
                let val = robot
                    .vm_state
                    .stack
                    .pop()
                    .map_err(|_| VMFault::StackUnderflow)?;
                robot
                    .vm_state
                    .stack
                    .push(val.sqrt())
                    .map_err(|_| VMFault::StackOverflow)
            }
            Instruction::Log => {
                let val = robot
                    .vm_state
                    .stack
                    .pop()
                    .map_err(|_| VMFault::StackUnderflow)?;
                robot
                    .vm_state
                    .stack
                    .push(val.ln())
                    .map_err(|_| VMFault::StackOverflow)
            }

            // Register-based arithmetic operations
            Instruction::AddOp(left, right) => {
                let left_val = left.get_value(&robot.vm_state)?;
                let right_val = right.get_value(&robot.vm_state)?;
                let result_val = left_val + right_val;
                robot
                    .vm_state
                    .registers
                    .set(Register::Result, result_val)
                    .map_err(|_| VMFault::PermissionError)
            }
            Instruction::SubOp(left, right) => {
                let left_val = left.get_value(&robot.vm_state)?;
                let right_val = right.get_value(&robot.vm_state)?;
                let result_val = left_val - right_val;
                robot
                    .vm_state
                    .registers
                    .set(Register::Result, result_val)
                    .map_err(|_| VMFault::PermissionError)
            }
            Instruction::MulOp(left, right) => {
                let left_val = left.get_value(&robot.vm_state)?;
                let right_val = right.get_value(&robot.vm_state)?;
                let result_val = left_val * right_val;
                // Log the debug value
                crate::debug_instructions!(
                    robot.id,
                    robot.vm_state.turn,
                    robot.vm_state.cycle,
                    "MulOp instruction: {} * {} = {}",
                    left_val,
                    right_val,
                    result_val,
                );
                robot
                    .vm_state
                    .registers
                    .set(Register::Result, result_val)
                    .map_err(|_| VMFault::PermissionError)
            }
            Instruction::DivOp(left, right) => {
                let left_val = left.get_value(&robot.vm_state)?;
                let right_val = right.get_value(&robot.vm_state)?;
                if right_val == 0.0 {
                    return Err(VMFault::DivisionByZero);
                }
                let result_val = left_val / right_val;
                robot
                    .vm_state
                    .registers
                    .set(Register::Result, result_val)
                    .map_err(|_| VMFault::PermissionError)
            }
            Instruction::ModOp(left, right) => {
                let left_val = left.get_value(&robot.vm_state)?;
                let right_val = right.get_value(&robot.vm_state)?;
                if right_val == 0.0 {
                    return Err(VMFault::DivisionByZero);
                }
                let result_val = left_val % right_val;
                robot
                    .vm_state
                    .registers
                    .set(Register::Result, result_val)
                    .map_err(|_| VMFault::PermissionError)
            }
            Instruction::PowOp(base_op, exp_op) => {
                let base = base_op.get_value(&robot.vm_state)?;
                let exponent = exp_op.get_value(&robot.vm_state)?;
                let result_val = base.powf(exponent);
                robot
                    .vm_state
                    .registers
                    .set(Register::Result, result_val)
                    .map_err(|_| VMFault::PermissionError)
            }
            Instruction::SqrtOp(op) => {
                let val = op.get_value(&robot.vm_state)?;
                let result_val = val.sqrt();
                robot
                    .vm_state
                    .registers
                    .set(Register::Result, result_val)
                    .map_err(|_| VMFault::PermissionError)
            }
            Instruction::LogOp(op) => {
                let val = op.get_value(&robot.vm_state)?;
                let result_val = val.ln(); // Natural log
                robot
                    .vm_state
                    .registers
                    .set(Register::Result, result_val)
                    .map_err(|_| VMFault::PermissionError)
            }
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
    use crate::vm::executor::InstructionExecutor;
    use crate::vm::executor::arithmetic_ops::ArithmeticOperations;
    use crate::vm::executor::processor::InstructionProcessor;
    use crate::vm::instruction::Instruction;
    use crate::vm::operand::Operand;
    use crate::vm::registers::Register;
    use std::collections::VecDeque;

    fn setup() -> (Robot, Arena, VecDeque<ArenaCommand>) {
        let arena = Arena::new();
        let center = Point {
            x: arena.width / 2.0,
            y: arena.height / 2.0,
        };
        let mut robot = Robot::new(0, "TestRobot".to_string(), Point { x: 0.5, y: 0.5 }, center);
        let command_queue = VecDeque::new();

        // Initialize registers for testing
        robot.vm_state.registers.set(Register::D0, 5.0).unwrap();
        robot.vm_state.registers.set(Register::D1, 10.0).unwrap();

        (robot, arena, command_queue)
    }

    #[test]
    fn test_can_process() {
        let processor = ArithmeticOperations::new();

        // Stack-based arithmetic operations
        assert!(processor.can_process(&Instruction::Add));
        assert!(processor.can_process(&Instruction::Sub));
        assert!(processor.can_process(&Instruction::Mul));
        assert!(processor.can_process(&Instruction::Div));
        assert!(processor.can_process(&Instruction::Mod));
        assert!(processor.can_process(&Instruction::Divmod));
        assert!(processor.can_process(&Instruction::Pow));
        assert!(processor.can_process(&Instruction::Sqrt));
        assert!(processor.can_process(&Instruction::Log));

        // Register-based arithmetic operations
        assert!(processor.can_process(&Instruction::AddOp(
            Operand::Value(1.0),
            Operand::Value(2.0)
        )));
        assert!(processor.can_process(&Instruction::SubOp(
            Operand::Value(1.0),
            Operand::Value(2.0)
        )));
        assert!(processor.can_process(&Instruction::MulOp(
            Operand::Value(1.0),
            Operand::Value(2.0)
        )));
        assert!(processor.can_process(&Instruction::DivOp(
            Operand::Value(1.0),
            Operand::Value(2.0)
        )));
        assert!(processor.can_process(&Instruction::ModOp(
            Operand::Value(1.0),
            Operand::Value(2.0)
        )));
        assert!(processor.can_process(&Instruction::PowOp(
            Operand::Value(1.0),
            Operand::Value(2.0)
        )));
        assert!(processor.can_process(&Instruction::SqrtOp(Operand::Value(1.0))));
        assert!(processor.can_process(&Instruction::LogOp(Operand::Value(1.0))));

        // Should not process non-arithmetic operations
        assert!(!processor.can_process(&Instruction::Push(Operand::Value(1.0))));
        assert!(!processor.can_process(&Instruction::Nop));
    }

    // Stack-based arithmetic operation tests

    #[test]
    fn test_add() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ArithmeticOperations::new();
        let all_robots = vec![];

        // Push values onto the stack
        robot.vm_state.stack.push(3.0).unwrap();
        robot.vm_state.stack.push(4.0).unwrap();

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Add,
            &mut command_queue,
        );

        assert!(result.is_ok());
        // Result should be 3.0 + 4.0 = 7.0
        assert_eq!(robot.vm_state.stack.pop().unwrap(), 7.0);
    }

    #[test]
    fn test_sub() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ArithmeticOperations::new();
        let all_robots = vec![];

        // Push values onto the stack
        robot.vm_state.stack.push(8.0).unwrap();
        robot.vm_state.stack.push(3.0).unwrap();

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Sub,
            &mut command_queue,
        );

        assert!(result.is_ok());
        // Result should be 8.0 - 3.0 = 5.0
        assert_eq!(robot.vm_state.stack.pop().unwrap(), 5.0);
    }

    #[test]
    fn test_mul() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ArithmeticOperations::new();
        let all_robots = vec![];

        // Push values onto the stack
        robot.vm_state.stack.push(5.0).unwrap();
        robot.vm_state.stack.push(4.0).unwrap();

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Mul,
            &mut command_queue,
        );

        assert!(result.is_ok());
        // Result should be 5.0 * 4.0 = 20.0
        assert_eq!(robot.vm_state.stack.pop().unwrap(), 20.0);
    }

    #[test]
    fn test_div() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ArithmeticOperations::new();
        let all_robots = vec![];

        // Push values onto the stack
        robot.vm_state.stack.push(20.0).unwrap();
        robot.vm_state.stack.push(4.0).unwrap();

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Div,
            &mut command_queue,
        );

        assert!(result.is_ok());
        // Result should be 20.0 / 4.0 = 5.0
        assert_eq!(robot.vm_state.stack.pop().unwrap(), 5.0);
    }

    #[test]
    fn test_div_by_zero() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ArithmeticOperations::new();
        let all_robots = vec![];

        // Push values onto the stack
        robot.vm_state.stack.push(20.0).unwrap();
        robot.vm_state.stack.push(0.0).unwrap();

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Div,
            &mut command_queue,
        );

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VMFault::DivisionByZero));
    }

    #[test]
    fn test_mod() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ArithmeticOperations::new();
        let all_robots = vec![];

        // Push values onto the stack
        robot.vm_state.stack.push(23.0).unwrap();
        robot.vm_state.stack.push(5.0).unwrap();

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Mod,
            &mut command_queue,
        );

        assert!(result.is_ok());
        // Result should be 23.0 % 5.0 = 3.0
        assert_eq!(robot.vm_state.stack.pop().unwrap(), 3.0);
    }

    #[test]
    fn test_divmod() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ArithmeticOperations::new();
        let all_robots = vec![];

        // Push values onto the stack
        robot.vm_state.stack.push(23.0).unwrap();
        robot.vm_state.stack.push(5.0).unwrap();

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Divmod,
            &mut command_queue,
        );

        assert!(result.is_ok());
        // Result should be quotient 4.0 and remainder 3.0
        assert_eq!(robot.vm_state.stack.pop().unwrap(), 4.0); // quotient
        assert_eq!(robot.vm_state.stack.pop().unwrap(), 3.0); // remainder
    }

    #[test]
    fn test_pow() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ArithmeticOperations::new();
        let all_robots = vec![];

        // Push values onto the stack
        robot.vm_state.stack.push(2.0).unwrap();
        robot.vm_state.stack.push(3.0).unwrap();

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Pow,
            &mut command_queue,
        );

        assert!(result.is_ok());
        // Result should be 2.0^3.0 = 8.0
        assert_eq!(robot.vm_state.stack.pop().unwrap(), 8.0);
    }

    #[test]
    fn test_sqrt() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ArithmeticOperations::new();
        let all_robots = vec![];

        // Push value onto the stack
        robot.vm_state.stack.push(16.0).unwrap();

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Sqrt,
            &mut command_queue,
        );

        assert!(result.is_ok());
        // Result should be sqrt(16.0) = 4.0
        assert_eq!(robot.vm_state.stack.pop().unwrap(), 4.0);
    }

    #[test]
    fn test_log() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ArithmeticOperations::new();
        let all_robots = vec![];

        // Push value onto the stack
        robot.vm_state.stack.push(std::f64::consts::E).unwrap(); // e

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Log,
            &mut command_queue,
        );

        assert!(result.is_ok());
        // Result should be ln(e) = 1.0
        assert!((robot.vm_state.stack.pop().unwrap() - 1.0).abs() < 1e-10); // Using approximate equality for floating-point
    }

    // Register-based arithmetic operation tests

    #[test]
    fn test_add_op() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ArithmeticOperations::new();
        let all_robots = vec![];

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::AddOp(Operand::Value(3.0), Operand::Value(4.0)),
            &mut command_queue,
        );

        assert!(result.is_ok());
        // Result should be 3.0 + 4.0 = 7.0 in Result register
        assert_eq!(robot.vm_state.registers.get(Register::Result).unwrap(), 7.0);
    }

    #[test]
    fn test_sub_op() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ArithmeticOperations::new();
        let all_robots = vec![];

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::SubOp(Operand::Value(8.0), Operand::Value(3.0)),
            &mut command_queue,
        );

        assert!(result.is_ok());
        // Result should be 8.0 - 3.0 = 5.0 in Result register
        assert_eq!(robot.vm_state.registers.get(Register::Result).unwrap(), 5.0);
    }

    #[test]
    fn test_mul_op() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ArithmeticOperations::new();
        let all_robots = vec![];

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::MulOp(Operand::Value(5.0), Operand::Value(4.0)),
            &mut command_queue,
        );

        assert!(result.is_ok());
        // Result should be 5.0 * 4.0 = 20.0 in Result register
        assert_eq!(
            robot.vm_state.registers.get(Register::Result).unwrap(),
            20.0
        );
    }

    #[test]
    fn test_div_op() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ArithmeticOperations::new();
        let all_robots = vec![];

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::DivOp(Operand::Value(20.0), Operand::Value(4.0)),
            &mut command_queue,
        );

        assert!(result.is_ok());
        // Result should be 20.0 / 4.0 = 5.0 in Result register
        assert_eq!(robot.vm_state.registers.get(Register::Result).unwrap(), 5.0);
    }

    #[test]
    fn test_div_op_by_zero() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ArithmeticOperations::new();
        let all_robots = vec![];

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::DivOp(Operand::Value(20.0), Operand::Value(0.0)),
            &mut command_queue,
        );

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VMFault::DivisionByZero));
    }

    #[test]
    fn test_mod_op() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ArithmeticOperations::new();
        let all_robots = vec![];

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::ModOp(Operand::Value(23.0), Operand::Value(5.0)),
            &mut command_queue,
        );

        assert!(result.is_ok());
        // Result should be 23.0 % 5.0 = 3.0 in Result register
        assert_eq!(robot.vm_state.registers.get(Register::Result).unwrap(), 3.0);
    }

    #[test]
    fn test_pow_op() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ArithmeticOperations::new();
        let all_robots = vec![];

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::PowOp(Operand::Value(2.0), Operand::Value(3.0)),
            &mut command_queue,
        );

        assert!(result.is_ok());
        // Result should be 2.0^3.0 = 8.0 in Result register
        assert_eq!(robot.vm_state.registers.get(Register::Result).unwrap(), 8.0);
    }

    #[test]
    fn test_sqrt_op() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ArithmeticOperations::new();
        let all_robots = vec![];

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::SqrtOp(Operand::Value(16.0)),
            &mut command_queue,
        );

        assert!(result.is_ok());
        // Result should be sqrt(16.0) = 4.0 in Result register
        assert_eq!(robot.vm_state.registers.get(Register::Result).unwrap(), 4.0);
    }

    #[test]
    fn test_log_op() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ArithmeticOperations::new();
        let all_robots = vec![];

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::LogOp(Operand::Value(std::f64::consts::E)), // e
            &mut command_queue,
        );

        assert!(result.is_ok());
        // Result should be ln(e) = 1.0 in Result register
        assert!((robot.vm_state.registers.get(Register::Result).unwrap() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_divmod_operation_integration() {
        let arena = Arena::new();
        let center = Point {
            x: arena.width / 2.0,
            y: arena.height / 2.0,
        };
        let mut robot = Robot::new(0, "TestRobot".to_string(), Point { x: 0.0, y: 0.0 }, center);
        let robots = vec![robot.clone()];
        let mut q = VecDeque::new();
        let executor = InstructionExecutor::new();

        // Test DIVMOD operation
        assert!(matches!(
            executor.execute_instruction(
                &mut robot,
                &robots,
                &arena,
                &Instruction::Push(Operand::Value(17.0)),
                &mut q
            ),
            Ok(())
        ));
        assert!(matches!(
            executor.execute_instruction(
                &mut robot,
                &robots,
                &arena,
                &Instruction::Push(Operand::Value(5.0)),
                &mut q
            ),
            Ok(())
        ));
        assert!(matches!(
            executor.execute_instruction(&mut robot, &robots, &arena, &Instruction::Divmod, &mut q),
            Ok(())
        ));

        // Popping gives quotient first (3)
        assert_eq!(robot.vm_state.stack.pop().unwrap(), 3.0);
        // Then remainder (2)
        assert_eq!(robot.vm_state.stack.pop().unwrap(), 2.0);

        // Test division by zero
        assert!(matches!(
            executor.execute_instruction(
                &mut robot,
                &robots,
                &arena,
                &Instruction::Push(Operand::Value(10.0)),
                &mut q
            ),
            Ok(())
        ));
        assert!(matches!(
            executor.execute_instruction(
                &mut robot,
                &robots,
                &arena,
                &Instruction::Push(Operand::Value(0.0)),
                &mut q
            ),
            Ok(())
        ));
        assert!(matches!(
            executor.execute_instruction(&mut robot, &robots, &arena, &Instruction::Divmod, &mut q),
            Err(VMFault::DivisionByZero)
        ));

        assert!(q.is_empty());
    }
}
