use crate::arena::Arena;
use crate::robot::Robot;
use crate::types::ArenaCommand;
use std::collections::VecDeque;

use super::processor::InstructionProcessor;
use crate::vm::error::VMFault;
use crate::vm::instruction::Instruction;
use crate::vm::registers::Register;

/// Processor for control flow operations
pub struct ControlFlowOperations;

impl ControlFlowOperations {
    pub fn new() -> Self {
        ControlFlowOperations
    }
}

impl InstructionProcessor for ControlFlowOperations {
    fn can_process(&self, instruction: &Instruction) -> bool {
        matches!(
            instruction,
            Instruction::Jmp(_)
                | Instruction::Jz(_)
                | Instruction::Jnz(_)
                | Instruction::Jl(_)
                | Instruction::Jle(_)
                | Instruction::Jg(_)
                | Instruction::Jge(_)
                | Instruction::Call(_)
                | Instruction::Ret
                | Instruction::Loop(_)
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
        let current_result_reg = robot
            .vm_state
            .registers
            .get(Register::Result)
            .unwrap_or(0.0);

        match instruction {
            Instruction::Jmp(target) => {
                crate::debug_instructions!(
                    robot.id,
                    robot.vm_state.turn,
                    robot.vm_state.cycle,
                    "Jmp: Jumping to {}",
                    target
                );
                robot.vm_state.ip = *target;
                Ok(())
            }
            Instruction::Jz(target) => {
                let is = (current_result_reg - 0.0).abs() < f64::EPSILON;
                crate::debug_instructions!(
                    robot.id,
                    robot.vm_state.turn,
                    robot.vm_state.cycle,
                    "Jz: @result = {:.4}. Jumping to {}? {}",
                    current_result_reg,
                    target,
                    is,
                );
                if is {
                    robot.vm_state.ip = *target;
                } else {
                    robot.vm_state.advance_ip();
                }
                Ok(())
            }
            Instruction::Jnz(target) => {
                let is = (current_result_reg - 0.0).abs() >= f64::EPSILON;
                crate::debug_instructions!(
                    robot.id,
                    robot.vm_state.turn,
                    robot.vm_state.cycle,
                    "Jnz: @result = {:.4}. Jumping to {}? {}",
                    current_result_reg,
                    target,
                    is,
                );
                if is {
                    robot.vm_state.ip = *target;
                } else {
                    robot.vm_state.advance_ip();
                }
                Ok(())
            }
            Instruction::Jl(target) => {
                crate::debug_instructions!(
                    robot.id,
                    robot.vm_state.turn,
                    robot.vm_state.cycle,
                    "Jl: @result = {:.4}. Jumping to {}? {}",
                    current_result_reg,
                    target,
                    current_result_reg < 0.0
                );
                if current_result_reg < 0.0 {
                    robot.vm_state.ip = *target;
                } else {
                    robot.vm_state.advance_ip();
                }
                Ok(())
            }
            Instruction::Jle(target) => {
                crate::debug_instructions!(
                    robot.id,
                    robot.vm_state.turn,
                    robot.vm_state.cycle,
                    "Jle: @result = {:.4}. Jumping to {}? {}",
                    current_result_reg,
                    target,
                    current_result_reg <= 0.0
                );
                if current_result_reg <= 0.0 {
                    robot.vm_state.ip = *target;
                } else {
                    robot.vm_state.advance_ip();
                }
                Ok(())
            }
            Instruction::Jg(target) => {
                crate::debug_instructions!(
                    robot.id,
                    robot.vm_state.turn,
                    robot.vm_state.cycle,
                    "Jg: @result = {:.4}. Jumping to {}? {}",
                    current_result_reg,
                    target,
                    current_result_reg > 0.0
                );
                if current_result_reg > 0.0 {
                    robot.vm_state.ip = *target;
                } else {
                    robot.vm_state.advance_ip();
                }
                Ok(())
            }
            Instruction::Jge(target) => {
                crate::debug_instructions!(
                    robot.id,
                    robot.vm_state.turn,
                    robot.vm_state.cycle,
                    "Jge: @result = {:.4}. Jumping to {}? {}",
                    current_result_reg,
                    target,
                    current_result_reg >= 0.0
                );
                if current_result_reg >= 0.0 {
                    robot.vm_state.ip = *target;
                } else {
                    robot.vm_state.advance_ip();
                }
                Ok(())
            }
            Instruction::Call(target) => {
                // Store the address of the next instruction (current IP + 1)
                let return_address = robot.vm_state.ip + 1;

                crate::debug_instructions!(
                    robot.id,
                    robot.vm_state.turn,
                    robot.vm_state.cycle,
                    "executing CALL. Pushing return addr {} and jumping to target {}",
                    return_address,
                    *target
                );

                // Push the return address to the call stack
                match robot.vm_state.push_call_stack(return_address) {
                    Ok(()) => {
                        // Jump to the target instruction
                        robot.vm_state.ip = *target;
                        Ok(())
                    }
                    Err(fault) => {
                        // Call stack overflow
                        crate::debug_instructions!(
                            robot.id,
                            robot.vm_state.turn,
                            robot.vm_state.cycle,
                            "CALL FAILED: {:?}",
                            fault
                        );
                        robot.vm_state.advance_ip();
                        Err(fault)
                    }
                }
            }
            Instruction::Ret => {
                // Pop the return address from the call stack
                crate::debug_instructions!(
                    robot.id,
                    robot.vm_state.turn,
                    robot.vm_state.cycle,
                    "executing RET. Current IP: {}, Call stack: {:?}",
                    robot.vm_state.ip,
                    robot.vm_state.call_stack
                );

                match robot.vm_state.pop_call_stack() {
                    Ok(return_address) => {
                        // Jump to the return address
                        robot.vm_state.advance_ip();
                        crate::debug_instructions!(
                            robot.id,
                            robot.vm_state.turn,
                            robot.vm_state.cycle,
                            "RET success. Popped return addr {}, jumping there",
                            return_address
                        );
                        robot.vm_state.ip = return_address;
                        Ok(())
                    }
                    Err(fault) => {
                        // Call stack underflow
                        crate::debug_instructions!(
                            robot.id,
                            robot.vm_state.turn,
                            robot.vm_state.cycle,
                            "RET FAILED: {:?}",
                            fault
                        );
                        robot.vm_state.advance_ip();
                        Err(fault)
                    }
                }
            }
            Instruction::Loop(target) => {
                // Get the current loop counter value
                let current_c = robot.vm_state.registers.get(Register::C).unwrap_or(0.0);
                let next_c = current_c - 1.0;
                crate::debug_instructions!(
                    robot.id,
                    robot.vm_state.turn,
                    robot.vm_state.cycle,
                    "executing LOOP. Current @c: {}, Next @c: {}",
                    current_c,
                    next_c
                );

                // Decrement the loop counter
                robot
                    .vm_state
                    .registers
                    .set(Register::C, next_c)
                    .map_err(|_| VMFault::PermissionError)?;

                // Jump if the *new* value of @c is not zero
                if next_c != 0.0 {
                    crate::debug_instructions!(
                        robot.id,
                        robot.vm_state.turn,
                        robot.vm_state.cycle,
                        "LOOP jumping to target {}",
                        *target
                    );
                    robot.vm_state.ip = *target;
                } else {
                    crate::debug_instructions!(
                        robot.id,
                        robot.vm_state.turn,
                        robot.vm_state.cycle,
                        "LOOP ended (c=0), continuing to next instruction"
                    );
                }
                Ok(())
            }
            _ => Err(VMFault::InvalidInstruction),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::Arena;
    use crate::robot::Robot;
    use crate::types::Point;
    use crate::vm::instruction::Instruction;
    use std::collections::VecDeque;

    fn setup() -> (Robot, Arena, VecDeque<ArenaCommand>) {
        let arena = Arena::new();
        let center = Point {
            x: arena.width / 2.0,
            y: arena.height / 2.0,
        };
        let mut robot = Robot::new(0, "TestRobot".to_string(), Point { x: 0.5, y: 0.5 }, center);
        let command_queue = VecDeque::new();

        // Initialize the result register for conditional jumps
        robot.vm_state.registers.set(Register::Result, 0.0).unwrap();
        // Initialize IP for testing jumps and calls
        robot.vm_state.ip = 10;

        (robot, arena, command_queue)
    }

    fn setup_call_ret_vm() -> (Robot, Arena, VecDeque<ArenaCommand>) {
        // Initialize with default state
        let arena = Arena::new();
        let center = Point {
            x: arena.width / 2.0,
            y: arena.height / 2.0,
        };
        let robot = Robot::new(0, "TestRobot".to_string(), Point { x: 0.0, y: 0.0 }, center);
        // robot.vm_state.sp = 1024; // TODO: SP access needs update if required
        let command_queue = VecDeque::new();
        (robot, arena, command_queue)
    }

    #[test]
    fn test_can_process() {
        let processor = ControlFlowOperations::new();

        assert!(processor.can_process(&Instruction::Jmp(0)));
        assert!(processor.can_process(&Instruction::Jz(0)));
        assert!(processor.can_process(&Instruction::Jnz(0)));
        assert!(processor.can_process(&Instruction::Jl(0)));
        assert!(processor.can_process(&Instruction::Jle(0)));
        assert!(processor.can_process(&Instruction::Jg(0)));
        assert!(processor.can_process(&Instruction::Jge(0)));
        assert!(processor.can_process(&Instruction::Call(0)));
        assert!(processor.can_process(&Instruction::Ret));
        assert!(processor.can_process(&Instruction::Loop(0)));

        // Should not process other operations
        assert!(!processor.can_process(&Instruction::Add));
        assert!(
            !processor.can_process(&Instruction::Push(crate::vm::operand::Operand::Value(0.0)))
        );
    }

    // Unconditional jump tests

    #[test]
    fn test_jmp() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ControlFlowOperations::new();
        let all_robots = vec![];

        let target_address = 42;

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Jmp(target_address),
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_eq!(robot.vm_state.ip, target_address);
    }

    // Conditional jump tests

    #[test]
    fn test_jz_taken() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ControlFlowOperations::new();
        let all_robots = vec![];

        // Set result register to 0 to ensure jump is taken
        robot.vm_state.registers.set(Register::Result, 0.0).unwrap();
        let target_address = 42;

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Jz(target_address),
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_eq!(
            robot.vm_state.ip, target_address,
            "Jump should be taken when @result = 0"
        );
    }

    #[test]
    fn test_jz_not_taken() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ControlFlowOperations::new();
        let all_robots = vec![];

        // Set result register to non-zero to ensure jump is not taken
        robot.vm_state.registers.set(Register::Result, 1.0).unwrap();
        let initial_ip = robot.vm_state.ip;
        let target_address = 42;

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Jz(target_address),
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_eq!(
            robot.vm_state.ip,
            initial_ip + 1,
            "Jump should not be taken when @result != 0"
        );
    }

    #[test]
    fn test_jnz_taken() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ControlFlowOperations::new();
        let all_robots = vec![];

        // Set result register to non-zero to ensure jump is taken
        robot.vm_state.registers.set(Register::Result, 1.0).unwrap();
        let target_address = 42;

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Jnz(target_address),
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_eq!(
            robot.vm_state.ip, target_address,
            "Jump should be taken when @result != 0"
        );
    }

    #[test]
    fn test_jnz_not_taken() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ControlFlowOperations::new();
        let all_robots = vec![];

        // Set result register to zero to ensure jump is not taken
        robot.vm_state.registers.set(Register::Result, 0.0).unwrap();
        let initial_ip = robot.vm_state.ip;
        let target_address = 42;

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Jnz(target_address),
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_eq!(
            robot.vm_state.ip,
            initial_ip + 1,
            "Jump should not be taken when @result = 0"
        );
    }

    #[test]
    fn test_jl_taken() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ControlFlowOperations::new();
        let all_robots = vec![];

        // Set result register to negative to ensure jump is taken
        robot
            .vm_state
            .registers
            .set(Register::Result, -1.0)
            .unwrap();
        let target_address = 42;

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Jl(target_address),
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_eq!(
            robot.vm_state.ip, target_address,
            "Jump should be taken when @result < 0"
        );
    }

    #[test]
    fn test_jl_not_taken() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ControlFlowOperations::new();
        let all_robots = vec![];

        // Set result register to zero to ensure jump is not taken
        robot.vm_state.registers.set(Register::Result, 0.0).unwrap();
        let initial_ip = robot.vm_state.ip;
        let target_address = 42;

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Jl(target_address),
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_eq!(
            robot.vm_state.ip,
            initial_ip + 1,
            "Jump should not be taken when @result >= 0"
        );
    }

    #[test]
    fn test_jle_taken_negative() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ControlFlowOperations::new();
        let all_robots = vec![];

        // Set result register to negative to ensure jump is taken
        robot
            .vm_state
            .registers
            .set(Register::Result, -1.0)
            .unwrap();
        let target_address = 42;

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Jle(target_address),
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_eq!(
            robot.vm_state.ip, target_address,
            "Jump should be taken when @result < 0"
        );
    }

    #[test]
    fn test_jle_taken_zero() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ControlFlowOperations::new();
        let all_robots = vec![];

        // Set result register to zero to ensure jump is taken
        robot.vm_state.registers.set(Register::Result, 0.0).unwrap();
        let target_address = 42;

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Jle(target_address),
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_eq!(
            robot.vm_state.ip, target_address,
            "Jump should be taken when @result = 0"
        );
    }

    #[test]
    fn test_jle_not_taken() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ControlFlowOperations::new();
        let all_robots = vec![];

        // Set result register to positive to ensure jump is not taken
        robot.vm_state.registers.set(Register::Result, 1.0).unwrap();
        let initial_ip = robot.vm_state.ip;
        let target_address = 42;

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Jle(target_address),
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_eq!(
            robot.vm_state.ip,
            initial_ip + 1,
            "Jump should not be taken when @result > 0"
        );
    }

    #[test]
    fn test_jg_taken() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ControlFlowOperations::new();
        let all_robots = vec![];

        // Set result register to positive to ensure jump is taken
        robot.vm_state.registers.set(Register::Result, 1.0).unwrap();
        let target_address = 42;

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Jg(target_address),
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_eq!(
            robot.vm_state.ip, target_address,
            "Jump should be taken when @result > 0"
        );
    }

    #[test]
    fn test_jg_not_taken() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ControlFlowOperations::new();
        let all_robots = vec![];

        // Set result register to zero to ensure jump is not taken
        robot.vm_state.registers.set(Register::Result, 0.0).unwrap();
        let initial_ip = robot.vm_state.ip;
        let target_address = 42;

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Jg(target_address),
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_eq!(
            robot.vm_state.ip,
            initial_ip + 1,
            "Jump should not be taken when @result <= 0"
        );
    }

    #[test]
    fn test_jge_taken_positive() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ControlFlowOperations::new();
        let all_robots = vec![];

        // Set result register to positive to ensure jump is taken
        robot.vm_state.registers.set(Register::Result, 1.0).unwrap();
        let target_address = 42;

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Jge(target_address),
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_eq!(
            robot.vm_state.ip, target_address,
            "Jump should be taken when @result > 0"
        );
    }

    #[test]
    fn test_jge_taken_zero() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ControlFlowOperations::new();
        let all_robots = vec![];

        // Set result register to zero to ensure jump is taken
        robot.vm_state.registers.set(Register::Result, 0.0).unwrap();
        let target_address = 42;

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Jge(target_address),
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_eq!(
            robot.vm_state.ip, target_address,
            "Jump should be taken when @result = 0"
        );
    }

    #[test]
    fn test_jge_not_taken() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ControlFlowOperations::new();
        let all_robots = vec![];

        // Set result register to negative to ensure jump is not taken
        robot
            .vm_state
            .registers
            .set(Register::Result, -1.0)
            .unwrap();
        let initial_ip = robot.vm_state.ip;
        let target_address = 42;

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Jge(target_address),
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_eq!(
            robot.vm_state.ip,
            initial_ip + 1,
            "Jump should not be taken when @result < 0"
        );
    }

    // Call and return tests

    #[test]
    fn test_call() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ControlFlowOperations::new();
        let all_robots = vec![];

        let initial_ip = robot.vm_state.ip;
        let target_address = 42;

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Call(target_address),
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_eq!(
            robot.vm_state.ip, target_address,
            "Call should set IP to target address"
        );

        // The call stack should contain the return address (initial_ip + 1)
        assert_eq!(
            robot.vm_state.call_stack.len(),
            1,
            "Call stack should have one entry"
        );
        assert_eq!(
            robot.vm_state.call_stack[0],
            initial_ip + 1,
            "Return address should be IP + 1"
        );
    }

    #[test]
    fn test_call_stack_overflow() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ControlFlowOperations::new();
        let all_robots = vec![];

        // Fill the call stack to capacity
        for i in 0..16 {
            // If push fails due to overflow, that's fine for this test
            let _ = robot.vm_state.push_call_stack(i);
        }

        let initial_ip = robot.vm_state.ip;
        let target_address = 42;

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Call(target_address),
            &mut command_queue,
        );

        assert_eq!(result, Err(VMFault::CallStackOverflow));
        assert_eq!(
            robot.vm_state.ip,
            initial_ip + 1,
            "IP should be incremented on call stack overflow"
        );
    }

    #[test]
    fn test_ret() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ControlFlowOperations::new();
        let all_robots = vec![];

        let return_address = 42;

        // Push a return address onto the call stack
        robot.vm_state.push_call_stack(return_address).unwrap();

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Ret,
            &mut command_queue,
        );

        assert!(result.is_ok());
        assert_eq!(
            robot.vm_state.ip, return_address,
            "Ret should set IP to return address"
        );
        assert_eq!(
            robot.vm_state.call_stack.len(),
            0,
            "Call stack should be empty after return"
        );
    }

    #[test]
    fn test_ret_stack_underflow() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ControlFlowOperations::new();
        let all_robots = vec![];

        let initial_ip = robot.vm_state.ip;

        // Call stack is empty
        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Ret,
            &mut command_queue,
        );

        assert_eq!(result, Err(VMFault::CallStackUnderflow));
        assert_eq!(
            robot.vm_state.ip,
            initial_ip + 1,
            "IP should be incremented on call stack underflow"
        );
    }

    // Loop tests

    #[test]
    fn test_loop_continue() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ControlFlowOperations::new();
        let all_robots = vec![];

        // Set the loop counter to 3
        robot.vm_state.registers.set(Register::C, 3.0).unwrap();

        let target_address = 5;

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Loop(target_address),
            &mut command_queue,
        );

        assert!(result.is_ok());
        // Loop counter should be decremented
        assert_eq!(robot.vm_state.registers.get(Register::C).unwrap(), 2.0);
        // IP should be set to target address
        assert_eq!(robot.vm_state.ip, target_address);
    }

    #[test]
    fn test_loop_end() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = ControlFlowOperations::new();
        let all_robots = vec![];

        // Set the loop counter to 1 (will decrement to 0)
        robot.vm_state.registers.set(Register::C, 1.0).unwrap();

        let initial_ip = robot.vm_state.ip;
        let target_address = 5;

        let result = processor.process(
            &mut robot,
            &all_robots,
            &arena,
            &Instruction::Loop(target_address),
            &mut command_queue,
        );

        assert!(result.is_ok());
        // Loop counter should be decremented to 0
        assert_eq!(robot.vm_state.registers.get(Register::C).unwrap(), 0.0);
        // IP should NOT be modified (the loop falls through)
        assert_eq!(robot.vm_state.ip, initial_ip);
    }

    #[test]
    fn test_call_ret_integration() {
        let (mut robot, arena, mut command_queue) = setup_call_ret_vm();
        let processor = ControlFlowOperations::new();

        let target_addr = 100.0;
        let original_ip = robot.vm_state.ip;

        let call_instruction = Instruction::Call(target_addr as usize);
        let result = processor.process(
            &mut robot,
            &[],
            &arena,
            &call_instruction,
            &mut command_queue,
        );
        assert!(result.is_ok());
        assert_eq!(robot.vm_state.ip, target_addr as usize);
        assert_eq!(robot.vm_state.call_stack.len(), 1);
        assert_eq!(robot.vm_state.call_stack[0], original_ip + 1);

        robot.vm_state.ip = 105; // Simulate execution at target
        let ret_instruction = Instruction::Ret;
        let result = processor.process(
            &mut robot,
            &[],
            &arena,
            &ret_instruction,
            &mut command_queue,
        );
        assert!(result.is_ok(), "Ret instruction failed");
        assert_eq!(robot.vm_state.ip, original_ip + 1);
        assert!(robot.vm_state.call_stack.is_empty());
    }
}
