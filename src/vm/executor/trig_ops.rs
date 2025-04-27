use crate::arena::Arena;
use crate::robot::Robot;
use crate::types::ArenaCommand;
use std::collections::VecDeque;

use super::processor::InstructionProcessor;
use crate::vm::error::VMFault;
use crate::vm::instruction::Instruction;
use crate::vm::registers::Register;

/// Processor for trigonometric and absolute value operations
pub struct TrigonometricOperations;

impl TrigonometricOperations {
    pub fn new() -> Self {
        TrigonometricOperations
    }
}

impl InstructionProcessor for TrigonometricOperations {
    fn can_process(&self, instruction: &Instruction) -> bool {
        matches!(
            instruction,
            // Stack-based trig operations
            Instruction::Sin |
            Instruction::Cos |
            Instruction::Tan |
            Instruction::Asin |
            Instruction::Acos |
            Instruction::Atan |
            Instruction::Atan2 |
            Instruction::Abs |
            // Register-based trig operations
            Instruction::SinOp(_) |
            Instruction::CosOp(_) |
            Instruction::TanOp(_) |
            Instruction::AsinOp(_) |
            Instruction::AcosOp(_) |
            Instruction::AtanOp(_) |
            Instruction::Atan2Op(_, _) |
            Instruction::AbsOp(_)
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
            // Stack-based operations
            Instruction::Sin => {
                let degrees = robot
                    .vm_state
                    .stack
                    .pop()
                    .map_err(|_| VMFault::StackUnderflow)?;
                robot
                    .vm_state
                    .stack
                    .push(degrees.to_radians().sin())
                    .map_err(|_| VMFault::StackOverflow)
            }
            Instruction::Cos => {
                let degrees = robot
                    .vm_state
                    .stack
                    .pop()
                    .map_err(|_| VMFault::StackUnderflow)?;
                robot
                    .vm_state
                    .stack
                    .push(degrees.to_radians().cos())
                    .map_err(|_| VMFault::StackOverflow)
            }
            Instruction::Tan => {
                let degrees = robot
                    .vm_state
                    .stack
                    .pop()
                    .map_err(|_| VMFault::StackUnderflow)?;
                robot
                    .vm_state
                    .stack
                    .push(degrees.to_radians().tan())
                    .map_err(|_| VMFault::StackOverflow)
            }
            Instruction::Asin => {
                let val = robot
                    .vm_state
                    .stack
                    .pop()
                    .map_err(|_| VMFault::StackUnderflow)?;
                robot
                    .vm_state
                    .stack
                    .push(val.asin().to_degrees())
                    .map_err(|_| VMFault::StackOverflow)
            }
            Instruction::Acos => {
                let val = robot
                    .vm_state
                    .stack
                    .pop()
                    .map_err(|_| VMFault::StackUnderflow)?;
                robot
                    .vm_state
                    .stack
                    .push(val.acos().to_degrees())
                    .map_err(|_| VMFault::StackOverflow)
            }
            Instruction::Atan => {
                let val = robot
                    .vm_state
                    .stack
                    .pop()
                    .map_err(|_| VMFault::StackUnderflow)?;
                robot
                    .vm_state
                    .stack
                    .push(val.atan().to_degrees())
                    .map_err(|_| VMFault::StackOverflow)
            }
            Instruction::Atan2 => {
                let x = robot
                    .vm_state
                    .stack
                    .pop()
                    .map_err(|_| VMFault::StackUnderflow)?;
                let y = robot
                    .vm_state
                    .stack
                    .pop()
                    .map_err(|_| VMFault::StackUnderflow)?;
                robot
                    .vm_state
                    .stack
                    .push(y.atan2(x).to_degrees())
                    .map_err(|_| VMFault::StackOverflow)
            }
            Instruction::Abs => {
                let val = robot
                    .vm_state
                    .stack
                    .pop()
                    .map_err(|_| VMFault::StackUnderflow)?;
                robot
                    .vm_state
                    .stack
                    .push(val.abs())
                    .map_err(|_| VMFault::StackOverflow)
            }

            // Register-based operations
            Instruction::SinOp(op) => {
                let degrees = op.get_value(&robot.vm_state)?;
                let result_val = degrees.to_radians().sin();
                robot
                    .vm_state
                    .registers
                    .set(Register::Result, result_val)
                    .map_err(|_| VMFault::PermissionError)
            }
            Instruction::CosOp(op) => {
                let degrees = op.get_value(&robot.vm_state)?;
                let result_val = degrees.to_radians().cos();
                robot
                    .vm_state
                    .registers
                    .set(Register::Result, result_val)
                    .map_err(|_| VMFault::PermissionError)
            }
            Instruction::TanOp(op) => {
                let degrees = op.get_value(&robot.vm_state)?;
                let result_val = degrees.to_radians().tan();
                robot
                    .vm_state
                    .registers
                    .set(Register::Result, result_val)
                    .map_err(|_| VMFault::PermissionError)
            }
            Instruction::AsinOp(op) => {
                let val = op.get_value(&robot.vm_state)?;
                let result_val = val.asin().to_degrees();
                robot
                    .vm_state
                    .registers
                    .set(Register::Result, result_val)
                    .map_err(|_| VMFault::PermissionError)
            }
            Instruction::AcosOp(op) => {
                let val = op.get_value(&robot.vm_state)?;
                let result_val = val.acos().to_degrees();
                robot
                    .vm_state
                    .registers
                    .set(Register::Result, result_val)
                    .map_err(|_| VMFault::PermissionError)
            }
            Instruction::AtanOp(op) => {
                let val = op.get_value(&robot.vm_state)?;
                let result_val = val.atan().to_degrees();
                robot
                    .vm_state
                    .registers
                    .set(Register::Result, result_val)
                    .map_err(|_| VMFault::PermissionError)
            }
            Instruction::Atan2Op(y_op, x_op) => {
                // Note: y comes first, consistent with stack atan2
                let y = y_op.get_value(&robot.vm_state)?;
                let x = x_op.get_value(&robot.vm_state)?;
                let result_val = y.atan2(x).to_degrees();
                robot
                    .vm_state
                    .registers
                    .set(Register::Result, result_val)
                    .map_err(|_| VMFault::PermissionError)
            }
            Instruction::AbsOp(op) => {
                let val = op.get_value(&robot.vm_state)?;
                let result_val = val.abs();
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

    use crate::vm::executor::processor::InstructionProcessor;
    use crate::vm::executor::trig_ops::TrigonometricOperations;
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
        robot.vm_state.registers.set(Register::D0, 30.0).unwrap(); // 30 degrees
        robot.vm_state.registers.set(Register::D1, 60.0).unwrap(); // 60 degrees

        (robot, arena, command_queue)
    }

    fn assert_approximately_equal(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-10, "Expected {}, got {}", b, a);
    }

    #[test]
    fn test_can_process() {
        let processor = TrigonometricOperations::new();

        // Stack-based operations
        assert!(processor.can_process(&Instruction::Sin));
        assert!(processor.can_process(&Instruction::Cos));
        assert!(processor.can_process(&Instruction::Tan));
        assert!(processor.can_process(&Instruction::Asin));
        assert!(processor.can_process(&Instruction::Acos));
        assert!(processor.can_process(&Instruction::Atan));
        assert!(processor.can_process(&Instruction::Atan2));
        assert!(processor.can_process(&Instruction::Abs));

        // Register-based operations
        assert!(processor.can_process(&Instruction::SinOp(Operand::Value(0.0))));
        assert!(processor.can_process(&Instruction::CosOp(Operand::Value(0.0))));
        assert!(processor.can_process(&Instruction::TanOp(Operand::Value(0.0))));
        assert!(processor.can_process(&Instruction::AsinOp(Operand::Value(0.0))));
        assert!(processor.can_process(&Instruction::AcosOp(Operand::Value(0.0))));
        assert!(processor.can_process(&Instruction::AtanOp(Operand::Value(0.0))));
        assert!(processor.can_process(&Instruction::Atan2Op(
            Operand::Value(0.0),
            Operand::Value(0.0)
        )));
        assert!(processor.can_process(&Instruction::AbsOp(Operand::Value(0.0))));

        // Should not process non-trig operations
        assert!(!processor.can_process(&Instruction::Add));
        assert!(!processor.can_process(&Instruction::Push(Operand::Value(0.0))));
    }

    // Stack-based operation tests

    #[test]
    fn test_sin() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = TrigonometricOperations::new();
        let all_robots = vec![];

        // Push 30 degrees onto the stack (sin(30°) = 0.5)
        robot.vm_state.stack.push(30.0).unwrap();

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Sin,
            &mut command_queue,
        );

        assert!(result.is_ok());
        // sin(30°) = 0.5
        assert_approximately_equal(robot.vm_state.stack.pop().unwrap(), 0.5);
    }

    #[test]
    fn test_cos() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = TrigonometricOperations::new();
        let all_robots = vec![];

        // Push 60 degrees onto the stack (cos(60°) = 0.5)
        robot.vm_state.stack.push(60.0).unwrap();

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Cos,
            &mut command_queue,
        );

        assert!(result.is_ok());
        // cos(60°) = 0.5
        assert_approximately_equal(robot.vm_state.stack.pop().unwrap(), 0.5);
    }

    #[test]
    fn test_tan() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = TrigonometricOperations::new();
        let all_robots = vec![];

        // Push 45 degrees onto the stack (tan(45°) = 1.0)
        robot.vm_state.stack.push(45.0).unwrap();

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Tan,
            &mut command_queue,
        );

        assert!(result.is_ok());
        // tan(45°) = 1.0
        assert_approximately_equal(robot.vm_state.stack.pop().unwrap(), 1.0);
    }

    #[test]
    fn test_asin() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = TrigonometricOperations::new();
        let all_robots = vec![];

        // Push 0.5 onto the stack (asin(0.5) = 30°)
        robot.vm_state.stack.push(0.5).unwrap();

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Asin,
            &mut command_queue,
        );

        assert!(result.is_ok());
        // asin(0.5) = 30°
        assert_approximately_equal(robot.vm_state.stack.pop().unwrap(), 30.0);
    }

    #[test]
    fn test_acos() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = TrigonometricOperations::new();
        let all_robots = vec![];

        // Push 0.5 onto the stack (acos(0.5) = 60°)
        robot.vm_state.stack.push(0.5).unwrap();

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Acos,
            &mut command_queue,
        );

        assert!(result.is_ok());
        // acos(0.5) = 60°
        assert_approximately_equal(robot.vm_state.stack.pop().unwrap(), 60.0);
    }

    #[test]
    fn test_atan() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = TrigonometricOperations::new();
        let all_robots = vec![];

        // Push 1.0 onto the stack (atan(1.0) = 45°)
        robot.vm_state.stack.push(1.0).unwrap();

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Atan,
            &mut command_queue,
        );

        assert!(result.is_ok());
        // atan(1.0) = 45°
        assert_approximately_equal(robot.vm_state.stack.pop().unwrap(), 45.0);
    }

    #[test]
    fn test_atan2() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = TrigonometricOperations::new();
        let all_robots = vec![];

        // Push y=1.0, x=1.0 onto the stack (atan2(1.0, 1.0) = 45°)
        robot.vm_state.stack.push(1.0).unwrap(); // y
        robot.vm_state.stack.push(1.0).unwrap(); // x

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Atan2,
            &mut command_queue,
        );

        assert!(result.is_ok());
        // atan2(1.0, 1.0) = 45°
        assert_approximately_equal(robot.vm_state.stack.pop().unwrap(), 45.0);
    }

    #[test]
    fn test_abs() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = TrigonometricOperations::new();
        let all_robots = vec![];

        // Push -5.0 onto the stack (abs(-5.0) = 5.0)
        robot.vm_state.stack.push(-5.0).unwrap();

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Abs,
            &mut command_queue,
        );

        assert!(result.is_ok());
        // abs(-5.0) = 5.0
        assert_eq!(robot.vm_state.stack.pop().unwrap(), 5.0);
    }

    // Register-based operation tests

    #[test]
    fn test_sin_op() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = TrigonometricOperations::new();
        let all_robots = vec![];

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::SinOp(Operand::Value(30.0)), // sin(30°) = 0.5
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_approximately_equal(robot.vm_state.registers.get(Register::Result).unwrap(), 0.5);
    }

    #[test]
    fn test_cos_op() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = TrigonometricOperations::new();
        let all_robots = vec![];

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::CosOp(Operand::Value(60.0)), // cos(60°) = 0.5
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_approximately_equal(robot.vm_state.registers.get(Register::Result).unwrap(), 0.5);
    }

    #[test]
    fn test_tan_op() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = TrigonometricOperations::new();
        let all_robots = vec![];

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::TanOp(Operand::Value(45.0)), // tan(45°) = 1.0
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_approximately_equal(robot.vm_state.registers.get(Register::Result).unwrap(), 1.0);
    }

    #[test]
    fn test_asin_op() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = TrigonometricOperations::new();
        let all_robots = vec![];

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::AsinOp(Operand::Value(0.5)), // asin(0.5) = 30°
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_approximately_equal(
            robot.vm_state.registers.get(Register::Result).unwrap(),
            30.0,
        );
    }

    #[test]
    fn test_acos_op() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = TrigonometricOperations::new();
        let all_robots = vec![];

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::AcosOp(Operand::Value(0.5)), // acos(0.5) = 60°
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_approximately_equal(
            robot.vm_state.registers.get(Register::Result).unwrap(),
            60.0,
        );
    }

    #[test]
    fn test_atan_op() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = TrigonometricOperations::new();
        let all_robots = vec![];

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::AtanOp(Operand::Value(1.0)), // atan(1.0) = 45°
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_approximately_equal(
            robot.vm_state.registers.get(Register::Result).unwrap(),
            45.0,
        );
    }

    #[test]
    fn test_atan2_op() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = TrigonometricOperations::new();
        let all_robots = vec![];

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Atan2Op(Operand::Value(1.0), Operand::Value(1.0)), // atan2(1.0, 1.0) = 45°
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_approximately_equal(
            robot.vm_state.registers.get(Register::Result).unwrap(),
            45.0,
        );
    }

    #[test]
    fn test_abs_op() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = TrigonometricOperations::new();
        let all_robots = vec![];

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::AbsOp(Operand::Value(-5.0)), // abs(-5.0) = 5.0
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_eq!(robot.vm_state.registers.get(Register::Result).unwrap(), 5.0);
    }

    #[test]
    fn test_register_operands() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = TrigonometricOperations::new();
        let all_robots = vec![];

        // Test using register as operand (D0 has 30.0 degrees)
        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::SinOp(Operand::Register(Register::D0)), // sin(30°) = 0.5
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_approximately_equal(robot.vm_state.registers.get(Register::Result).unwrap(), 0.5);
    }
}
