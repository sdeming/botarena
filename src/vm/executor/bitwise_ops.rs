use crate::arena::Arena;
use crate::robot::Robot;
use crate::types::ArenaCommand;
use std::collections::VecDeque;

use super::processor::InstructionProcessor;
use crate::vm::error::VMFault;
use crate::vm::instruction::Instruction;

/// Processor for bitwise operations
pub struct BitwiseOperations;

impl BitwiseOperations {
    pub fn new() -> Self {
        BitwiseOperations
    }
}

impl InstructionProcessor for BitwiseOperations {
    fn can_process(&self, instruction: &Instruction) -> bool {
        matches!(
            instruction,
            // Stack-based bitwise operations
            Instruction::And
                | Instruction::Or
                | Instruction::Xor
                | Instruction::Not
                | Instruction::Shl
                | Instruction::Shr
                // Operand-based bitwise operations
                | Instruction::AndOp(_, _)
                | Instruction::OrOp(_, _)
                | Instruction::XorOp(_, _)
                | Instruction::NotOp(_)
                | Instruction::ShlOp(_, _)
                | Instruction::ShrOp(_, _)
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
            // Stack-based bitwise operations
            Instruction::And => {
                let b = robot
                    .vm_state
                    .stack
                    .pop()
                    .map_err(|_| VMFault::StackUnderflow)? as u32;
                let a = robot
                    .vm_state
                    .stack
                    .pop()
                    .map_err(|_| VMFault::StackUnderflow)? as u32;
                let result = a & b;
                robot
                    .vm_state
                    .stack
                    .push(result as f64)
                    .map_err(|_| VMFault::StackOverflow)
            }
            Instruction::Or => {
                let b = robot
                    .vm_state
                    .stack
                    .pop()
                    .map_err(|_| VMFault::StackUnderflow)? as u32;
                let a = robot
                    .vm_state
                    .stack
                    .pop()
                    .map_err(|_| VMFault::StackUnderflow)? as u32;
                let result = a | b;
                robot
                    .vm_state
                    .stack
                    .push(result as f64)
                    .map_err(|_| VMFault::StackOverflow)
            }
            Instruction::Xor => {
                let b = robot
                    .vm_state
                    .stack
                    .pop()
                    .map_err(|_| VMFault::StackUnderflow)? as u32;
                let a = robot
                    .vm_state
                    .stack
                    .pop()
                    .map_err(|_| VMFault::StackUnderflow)? as u32;
                let result = a ^ b;
                robot
                    .vm_state
                    .stack
                    .push(result as f64)
                    .map_err(|_| VMFault::StackOverflow)
            }
            Instruction::Not => {
                let val = robot
                    .vm_state
                    .stack
                    .pop()
                    .map_err(|_| VMFault::StackUnderflow)? as u32;
                // Apply NOT operation
                let result = !val;
                // Keep result as unsigned to match integration test behavior
                robot
                    .vm_state
                    .stack
                    .push(result as f64)
                    .map_err(|_| VMFault::StackOverflow)
            }
            Instruction::Shl => {
                let shift = robot
                    .vm_state
                    .stack
                    .pop()
                    .map_err(|_| VMFault::StackUnderflow)? as i64;
                let val = robot
                    .vm_state
                    .stack
                    .pop()
                    .map_err(|_| VMFault::StackUnderflow)? as u32;

                // Ensure we don't attempt to shift by a negative amount
                if shift < 0 {
                    return Err(VMFault::DivisionByZero);
                }

                // Clamp shift amount to 31 bits
                let shift_amount = if shift > 31 { 31 } else { shift as u32 };

                let result = val << shift_amount;
                robot
                    .vm_state
                    .stack
                    .push(result as f64)
                    .map_err(|_| VMFault::StackOverflow)
            }
            Instruction::Shr => {
                let shift = robot
                    .vm_state
                    .stack
                    .pop()
                    .map_err(|_| VMFault::StackUnderflow)? as i64;
                let val = robot
                    .vm_state
                    .stack
                    .pop()
                    .map_err(|_| VMFault::StackUnderflow)? as u32;

                // Ensure we don't attempt to shift by a negative amount
                if shift < 0 {
                    return Err(VMFault::DivisionByZero);
                }

                // Clamp shift amount to 31 bits
                let shift_amount = if shift > 31 { 31 } else { shift as u32 };

                let result = val >> shift_amount;
                robot
                    .vm_state
                    .stack
                    .push(result as f64)
                    .map_err(|_| VMFault::StackOverflow)
            }
            
            // Operand-based bitwise operations
            Instruction::AndOp(left, right) => {
                let left_val = left.get_value(&robot.vm_state)? as u32;
                let right_val = right.get_value(&robot.vm_state)? as u32;
                let result_val = left_val & right_val;
                robot
                    .vm_state
                    .registers
                    .set(crate::vm::registers::Register::Result, result_val as f64)
                    .map_err(|_| VMFault::PermissionError)
            }
            Instruction::OrOp(left, right) => {
                let left_val = left.get_value(&robot.vm_state)? as u32;
                let right_val = right.get_value(&robot.vm_state)? as u32;
                let result_val = left_val | right_val;
                robot
                    .vm_state
                    .registers
                    .set(crate::vm::registers::Register::Result, result_val as f64)
                    .map_err(|_| VMFault::PermissionError)
            }
            Instruction::XorOp(left, right) => {
                let left_val = left.get_value(&robot.vm_state)? as u32;
                let right_val = right.get_value(&robot.vm_state)? as u32;
                let result_val = left_val ^ right_val;
                robot
                    .vm_state
                    .registers
                    .set(crate::vm::registers::Register::Result, result_val as f64)
                    .map_err(|_| VMFault::PermissionError)
            }
            Instruction::NotOp(op) => {
                let val = op.get_value(&robot.vm_state)? as u32;
                let result_val = !val;
                robot
                    .vm_state
                    .registers
                    .set(crate::vm::registers::Register::Result, result_val as f64)
                    .map_err(|_| VMFault::PermissionError)
            }
            Instruction::ShlOp(left, right) => {
                let val = left.get_value(&robot.vm_state)? as u32;
                let shift = right.get_value(&robot.vm_state)? as i64;
                
                // Ensure we don't attempt to shift by a negative amount
                if shift < 0 {
                    return Err(VMFault::DivisionByZero);
                }
                
                // Clamp shift amount to 31 bits
                let shift_amount = if shift > 31 { 31 } else { shift as u32 };
                
                let result_val = val << shift_amount;
                robot
                    .vm_state
                    .registers
                    .set(crate::vm::registers::Register::Result, result_val as f64)
                    .map_err(|_| VMFault::PermissionError)
            }
            Instruction::ShrOp(left, right) => {
                let val = left.get_value(&robot.vm_state)? as u32;
                let shift = right.get_value(&robot.vm_state)? as i64;
                
                // Ensure we don't attempt to shift by a negative amount
                if shift < 0 {
                    return Err(VMFault::DivisionByZero);
                }
                
                // Clamp shift amount to 31 bits
                let shift_amount = if shift > 31 { 31 } else { shift as u32 };
                
                let result_val = val >> shift_amount;
                robot
                    .vm_state
                    .registers
                    .set(crate::vm::registers::Register::Result, result_val as f64)
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
    use crate::vm::executor::bitwise_ops::BitwiseOperations;
    use crate::vm::executor::processor::InstructionProcessor;
    use crate::vm::instruction::Instruction;
    use crate::vm::operand::Operand;
    use crate::vm::registers::Register;
    use std::collections::VecDeque;

    fn setup() -> (Robot, Arena, VecDeque<ArenaCommand>) {
        let mut robot = Robot::new(0, "TestRobot".to_string(), Point { x: 0.5, y: 0.5 });
        let arena = Arena::new();
        let command_queue = VecDeque::new();
        
        // Initialize registers for testing
        robot.vm_state.registers.set(Register::D0, 5.0).unwrap(); // 0101 in binary
        robot.vm_state.registers.set(Register::D1, 3.0).unwrap(); // 0011 in binary
        
        (robot, arena, command_queue)
    }

    #[test]
    fn test_can_process() {
        let processor = BitwiseOperations::new();

        // Stack-based operations
        assert!(processor.can_process(&Instruction::And));
        assert!(processor.can_process(&Instruction::Or));
        assert!(processor.can_process(&Instruction::Xor));
        assert!(processor.can_process(&Instruction::Not));
        assert!(processor.can_process(&Instruction::Shl));
        assert!(processor.can_process(&Instruction::Shr));
        
        // Operand-based operations
        assert!(processor.can_process(&Instruction::AndOp(
            Operand::Value(1.0),
            Operand::Value(2.0)
        )));
        assert!(processor.can_process(&Instruction::OrOp(
            Operand::Value(1.0),
            Operand::Value(2.0)
        )));
        assert!(processor.can_process(&Instruction::XorOp(
            Operand::Value(1.0),
            Operand::Value(2.0)
        )));
        assert!(processor.can_process(&Instruction::NotOp(Operand::Value(1.0))));
        assert!(processor.can_process(&Instruction::ShlOp(
            Operand::Value(1.0),
            Operand::Value(2.0)
        )));
        assert!(processor.can_process(&Instruction::ShrOp(
            Operand::Value(1.0),
            Operand::Value(2.0)
        )));

        // Should not process other operations
        assert!(!processor.can_process(&Instruction::Add));
        assert!(
            !processor.can_process(&Instruction::Push(crate::vm::operand::Operand::Value(0.0)))
        );
    }
    
    // Tests for operand-based bitwise operations
    
    #[test]
    fn test_and_op() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = BitwiseOperations::new();
        let all_robots = vec![];

        // 5 & 3 = 1
        // 0101 & 0011 = 0001
        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::AndOp(Operand::Register(Register::D0), Operand::Register(Register::D1)),
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_eq!(robot.vm_state.registers.get(Register::Result).unwrap(), 1.0);
    }
    
    #[test]
    fn test_or_op() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = BitwiseOperations::new();
        let all_robots = vec![];

        // 5 | 3 = 7
        // 0101 | 0011 = 0111
        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::OrOp(Operand::Register(Register::D0), Operand::Register(Register::D1)),
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_eq!(robot.vm_state.registers.get(Register::Result).unwrap(), 7.0);
    }
    
    #[test]
    fn test_xor_op() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = BitwiseOperations::new();
        let all_robots = vec![];

        // 5 ^ 3 = 6
        // 0101 ^ 0011 = 0110
        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::XorOp(Operand::Register(Register::D0), Operand::Register(Register::D1)),
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_eq!(robot.vm_state.registers.get(Register::Result).unwrap(), 6.0);
    }
    
    #[test]
    fn test_not_op() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = BitwiseOperations::new();
        let all_robots = vec![];

        // ~5 = 4294967290
        // ~00000000 00000000 00000000 00000101 = 11111111 11111111 11111111 11111010
        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::NotOp(Operand::Register(Register::D0)),
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_eq!(robot.vm_state.registers.get(Register::Result).unwrap(), (!5u32) as f64);
    }
    
    #[test]
    fn test_shl_op() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = BitwiseOperations::new();
        let all_robots = vec![];

        // 5 << 1 = 10
        // 0101 << 1 = 1010
        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::ShlOp(Operand::Register(Register::D0), Operand::Value(1.0)),
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_eq!(robot.vm_state.registers.get(Register::Result).unwrap(), 10.0);
    }
    
    #[test]
    fn test_shr_op() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = BitwiseOperations::new();
        let all_robots = vec![];

        // 5 >> 1 = 2
        // 0101 >> 1 = 0010
        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::ShrOp(Operand::Register(Register::D0), Operand::Value(1.0)),
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_eq!(robot.vm_state.registers.get(Register::Result).unwrap(), 2.0);
    }
    
    #[test]
    fn test_negative_shift_op() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = BitwiseOperations::new();
        let all_robots = vec![];

        // Attempt to shift by a negative amount should fail
        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::ShlOp(Operand::Register(Register::D0), Operand::Value(-1.0)),
            &mut command_queue,
        );

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VMFault::DivisionByZero));
    }
    
    #[test]
    fn test_overflow_shift_op() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = BitwiseOperations::new();
        let all_robots = vec![];

        // Shift by more than 31 bits should be clamped to 31
        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::ShlOp(Operand::Register(Register::D0), Operand::Value(100.0)),
            &mut command_queue,
        );

        assert!(result.is_ok());
        // 5 << 31 (not 100) = 10737418240
        assert_eq!(robot.vm_state.registers.get(Register::Result).unwrap(), (5u32 << 31) as f64);
    }

    // Stack-based operation tests
    
    #[test]
    fn test_and() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = BitwiseOperations::new();
        let all_robots = vec![];

        // 5 & 3 = 1
        // 0101 & 0011 = 0001
        robot.vm_state.stack.push(5.0).unwrap();
        robot.vm_state.stack.push(3.0).unwrap();

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::And,
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_eq!(robot.vm_state.stack.pop().unwrap(), 1.0);
    }
    
    #[test]
    fn test_or() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = BitwiseOperations::new();
        let all_robots = vec![];

        // 5 | 3 = 7
        // 0101 | 0011 = 0111
        robot.vm_state.stack.push(5.0).unwrap();
        robot.vm_state.stack.push(3.0).unwrap();

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Or,
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_eq!(robot.vm_state.stack.pop().unwrap(), 7.0);
    }
    
    #[test]
    fn test_xor() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = BitwiseOperations::new();
        let all_robots = vec![];

        // 5 ^ 3 = 6
        // 0101 ^ 0011 = 0110
        robot.vm_state.stack.push(5.0).unwrap();
        robot.vm_state.stack.push(3.0).unwrap();

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Xor,
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_eq!(robot.vm_state.stack.pop().unwrap(), 6.0);
    }
    
    #[test]
    fn test_not() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = BitwiseOperations::new();
        let all_robots = vec![];

        // NOTE: Bitwise NOT in Rust works on the entire representation of the number
        // For a u32, ~5 would be all 1's except the 3 least significant bits (101)
        robot.vm_state.stack.push(5.0).unwrap();

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Not,
            &mut command_queue,
        );

        assert!(result.is_ok());

        // The expected value can be either -6 (when interpreted as signed) or 4294967290 (when unsigned)
        // For consistency with the integration test, we'll expect the unsigned value
        assert_eq!(robot.vm_state.stack.pop().unwrap(), (!5u32) as f64);
    }
    
    #[test]
    fn test_shl() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = BitwiseOperations::new();
        let all_robots = vec![];

        // 1 << 2 = 4
        // 01 << 2 = 0100
        robot.vm_state.stack.push(1.0).unwrap();
        robot.vm_state.stack.push(2.0).unwrap();

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Shl,
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_eq!(robot.vm_state.stack.pop().unwrap(), 4.0);
    }
    
    #[test]
    fn test_shr() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = BitwiseOperations::new();
        let all_robots = vec![];

        // 8 >> 2 = 2
        // 1000 >> 2 = 0010
        robot.vm_state.stack.push(8.0).unwrap();
        robot.vm_state.stack.push(2.0).unwrap();

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Shr,
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_eq!(robot.vm_state.stack.pop().unwrap(), 2.0);
    }
    
    #[test]
    fn test_negative_shift() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = BitwiseOperations::new();
        let all_robots = vec![];

        // Attempt to shift by a negative amount
        robot.vm_state.stack.push(8.0).unwrap();
        robot.vm_state.stack.push(-2.0).unwrap();

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Shl,
            &mut command_queue,
        );

        assert_eq!(result, Err(VMFault::DivisionByZero));
    }
    
    #[test]
    fn test_overflow_shift() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = BitwiseOperations::new();
        let all_robots = vec![];

        // Attempt to shift by too many bits
        robot.vm_state.stack.push(8.0).unwrap();
        robot.vm_state.stack.push(64.0).unwrap();

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Shr,
            &mut command_queue,
        );

        // For consistency with the integration test, we should expect this to succeed
        // with the shift amount clamped to 31
        assert!(result.is_ok());
        assert_eq!(robot.vm_state.stack.pop().unwrap(), 0.0); // 8 >> 31 = 0
    }

    #[test]
    fn test_stack_underflow() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = BitwiseOperations::new();
        let all_robots = vec![];

        // AND requires two operands, but the stack is empty
        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::And,
            &mut command_queue,
        );

        assert_eq!(result, Err(VMFault::StackUnderflow));
    }
}
