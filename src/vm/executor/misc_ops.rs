use crate::arena::Arena;
use crate::robot::Robot;
use crate::types::ArenaCommand;
use std::collections::VecDeque;

use super::processor::InstructionProcessor;
use crate::vm::error::VMFault;
use crate::vm::instruction::Instruction;

/// Processor for miscellaneous operations like Nop and Dbg
pub struct MiscellaneousOperations;

impl MiscellaneousOperations {
    pub fn new() -> Self {
        MiscellaneousOperations
    }
}

impl InstructionProcessor for MiscellaneousOperations {
    fn can_process(&self, instruction: &Instruction) -> bool {
        matches!(instruction, Instruction::Nop | Instruction::Dbg(_) | Instruction::Sleep(_))
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
            Instruction::Nop => {
                // No operation - advance IP and return
                robot.vm_state.advance_ip();
                Ok(())
            }
            Instruction::Dbg(op) => {
                // Get the value to debug from the operand
                let val = op.get_value(&robot.vm_state)?;

                // Log the debug value
                crate::debug_instructions!(
                    robot.id,
                    robot.vm_state.turn,
                    robot.vm_state.cycle,
                    "DBG instruction: {}",
                    val
                );

                // Advance IP and return
                robot.vm_state.advance_ip();
                Ok(())
            }
            Instruction::Sleep(op) => {
                let cycles = op.get_value(&robot.vm_state)?.max(1.0) as u32;
                // Set the remaining cycles for this instruction (minus one for the current cycle)
                robot.vm_state.instruction_cycles_remaining = cycles - 1;
                // Only advance IP after sleep completes (handled by VM cycle logic)
                Ok(())
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
    use crate::vm::executor::misc_ops::MiscellaneousOperations;
    use crate::vm::executor::processor::InstructionProcessor;
    use crate::vm::instruction::Instruction;
    use crate::vm::operand::Operand;
    use crate::vm::registers::Register;
    use std::collections::VecDeque;

    fn setup() -> (Robot, Arena, VecDeque<ArenaCommand>) {
        let robot = Robot::new(1, Point { x: 0.5, y: 0.5 });
        let arena = Arena::new();
        let command_queue = VecDeque::new();

        (robot, arena, command_queue)
    }

    #[test]
    fn test_can_process() {
        let processor = MiscellaneousOperations::new();

        // Should process miscellaneous operations
        assert!(processor.can_process(&Instruction::Nop));
        assert!(processor.can_process(&Instruction::Dbg(Operand::Value(1.0))));
        assert!(processor.can_process(&Instruction::Sleep(Operand::Value(1.0))));

        // Should not process other operations
        assert!(!processor.can_process(&Instruction::Push(Operand::Value(1.0))));
        assert!(!processor.can_process(&Instruction::Mov(Register::D0, Operand::Value(1.0))));
    }

    #[test]
    fn test_nop_instruction() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = MiscellaneousOperations::new();
        let all_robots = vec![];

        // Get the current IP
        let initial_ip = robot.vm_state.ip;

        // Execute Nop instruction
        let nop = Instruction::Nop;
        let result = processor.process(&mut robot, &all_robots, &arena, &nop, &mut command_queue);

        // Nop should succeed
        assert!(result.is_ok());

        // IP should have advanced by 1
        assert_eq!(robot.vm_state.ip, initial_ip + 1);

        // Command queue should still be empty
        assert_eq!(command_queue.len(), 0);
    }

    #[test]
    fn test_dbg_instruction_with_value() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = MiscellaneousOperations::new();
        let all_robots = vec![];

        // Get the current IP
        let initial_ip = robot.vm_state.ip;

        // Execute Dbg instruction with a constant value
        let dbg = Instruction::Dbg(Operand::Value(42.0));
        let result = processor.process(&mut robot, &all_robots, &arena, &dbg, &mut command_queue);

        // Dbg should succeed
        assert!(result.is_ok());

        // IP should have advanced by 1
        assert_eq!(robot.vm_state.ip, initial_ip + 1);

        // Command queue should still be empty (Dbg only logs, doesn't queue commands)
        assert_eq!(command_queue.len(), 0);
    }

    #[test]
    fn test_dbg_instruction_with_register() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = MiscellaneousOperations::new();
        let all_robots = vec![];

        // Set a value in a register
        robot.vm_state.registers.set(Register::D0, 123.0).unwrap();

        // Get the current IP
        let initial_ip = robot.vm_state.ip;

        // Execute Dbg instruction with a register operand
        let dbg = Instruction::Dbg(Operand::Register(Register::D0));
        let result = processor.process(&mut robot, &all_robots, &arena, &dbg, &mut command_queue);

        // Dbg should succeed
        assert!(result.is_ok());

        // IP should have advanced by 1
        assert_eq!(robot.vm_state.ip, initial_ip + 1);

        // Command queue should still be empty
        assert_eq!(command_queue.len(), 0);
    }

    #[test]
    fn test_sleep_instruction() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = MiscellaneousOperations::new();
        let all_robots = vec![];

        // Get the current IP
        let initial_ip = robot.vm_state.ip;

        // Execute Sleep instruction with a constant value
        let sleep = Instruction::Sleep(Operand::Value(3.0));
        let result = processor.process(&mut robot, &all_robots, &arena, &sleep, &mut command_queue);

        // Sleep should succeed
        assert!(result.is_ok());

        // After processing, instruction_cycles_remaining should be 2 (3-1)
        assert_eq!(robot.vm_state.instruction_cycles_remaining, 2);
        // IP should NOT have advanced yet
        assert_eq!(robot.vm_state.ip, initial_ip);

        // Simulate VM cycles: decrement instruction_cycles_remaining until 0
        while robot.vm_state.instruction_cycles_remaining > 0 {
            robot.vm_state.instruction_cycles_remaining -= 1;
            // IP should still not advance
            assert_eq!(robot.vm_state.ip, initial_ip);
        }

        // After sleep completes, the VM would advance the IP
        robot.vm_state.advance_ip();
        assert_eq!(robot.vm_state.ip, initial_ip + 1);

        // Command queue should still be empty (Sleep only waits, doesn't queue commands)
        assert_eq!(command_queue.len(), 0);
    }

    #[test]
    fn test_invalid_instruction() {
        let (mut robot, arena, mut command_queue) = setup();
        let processor = MiscellaneousOperations::new();
        let all_robots = vec![];

        // Try to execute an instruction that doesn't match any patterns in the process method
        let push = Instruction::Push(Operand::Value(1.0));
        let result = processor.process(&mut robot, &all_robots, &arena, &push, &mut command_queue);

        // Should return an error
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VMFault::InvalidInstruction));
    }
}
