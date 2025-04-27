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
        // Find a processor that can handle this instruction
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
    use crate::types::{Point, ArenaCommand};
    use crate::vm::instruction::Instruction;
    use crate::vm::operand::Operand;
    use crate::vm::error::VMFault;
    use crate::vm::registers::Register;
    use std::collections::VecDeque;
    use crate::vm::executor::InstructionExecutor;

    fn setup_test_vm() -> (Robot, Arena, VecDeque<ArenaCommand>) {
        let arena = Arena::new();
        let center = Point { x: arena.width / 2.0, y: arena.height / 2.0 };
        let robot = Robot::new(0, "TestRobot0".to_string(), Point { x: 0.5, y: 0.5 }, center);
        let command_queue = VecDeque::new();
        (robot, arena, command_queue)
    }

    fn execute_instruction(
        robot: &mut Robot,
        arena: &Arena,
        instruction: &Instruction,
        command_queue: &mut VecDeque<ArenaCommand>,
    ) -> Result<(), VMFault> {
        let executor = InstructionExecutor::new();
        let all_robots = vec![];
        executor.execute_instruction(robot, &all_robots, arena, instruction, command_queue)
    }

    #[test]
    fn test_stack_operations_delegation() {
        let arena = Arena::new();
        let center = Point { x: arena.width / 2.0, y: arena.height / 2.0 };
        let mut robot = Robot::new(0, "TestRobot0".to_string(), Point { x: 0.5, y: 0.5 }, center);
        let mut command_queue = VecDeque::new();

        let result_push = execute_instruction(
            &mut robot,
            &arena,
            &Instruction::Push(Operand::Value(42.0)),
            &mut command_queue,
        );
        assert!(result_push.is_ok(), "Push failed");

        let result_pop = execute_instruction(
            &mut robot,
            &arena,
            &Instruction::Pop(Register::D0),
            &mut command_queue,
        );
        assert!(result_pop.is_ok(), "Pop failed");
        assert_eq!(robot.vm_state.registers.get(Register::D0).unwrap(), 42.0);
    }

    #[test]
    fn test_register_operations_delegation() {
        let arena = Arena::new();
        let center = Point { x: arena.width / 2.0, y: arena.height / 2.0 };
        let mut robot = Robot::new(0, "TestRobot0".to_string(), Point { x: 0.5, y: 0.5 }, center);
        let mut command_queue = VecDeque::new();

        let result_mov = execute_instruction(
            &mut robot,
            &arena,
            &Instruction::Mov(Register::D1, Operand::Value(55.0)),
            &mut command_queue,
        );
        assert!(result_mov.is_ok(), "Mov failed");
        assert_eq!(robot.vm_state.registers.get(Register::D1).unwrap(), 55.0);

        let result_cmp = execute_instruction(
            &mut robot,
            &arena,
            &Instruction::Cmp(Operand::Value(10.0), Operand::Value(5.0)),
            &mut command_queue,
        );
        assert!(result_cmp.is_ok(), "Cmp failed");
        assert_eq!(robot.vm_state.registers.get(Register::Result).unwrap(), 5.0);
    }

    #[test]
    fn test_arithmetic_execution() {
        let (mut robot, arena, mut command_queue) = setup_test_vm();
        robot.vm_state.registers.set(Register::D1, 10.0).unwrap();
        robot.vm_state.registers.set(Register::D2, 5.0).unwrap();
        let instruction = Instruction::AddOp(Operand::Register(Register::D1), Operand::Register(Register::D2));
        execute_instruction(&mut robot, &arena, &instruction, &mut command_queue).unwrap();
        assert_eq!(robot.vm_state.registers.get(Register::Result).unwrap(), 15.0);
    }

    #[test]
    fn test_bitwise_execution() {
        let (mut robot, arena, mut command_queue) = setup_test_vm();
        robot.vm_state.registers.set(Register::D1, 0b1010 as f64).unwrap();
        robot.vm_state.registers.set(Register::D2, 0b1100 as f64).unwrap();
        execute_instruction(&mut robot, &arena, &Instruction::Push(Operand::Register(Register::D1)), &mut command_queue).unwrap();
        execute_instruction(&mut robot, &arena, &Instruction::Push(Operand::Register(Register::D2)), &mut command_queue).unwrap();
        execute_instruction(&mut robot, &arena, &Instruction::And, &mut command_queue).unwrap();
        execute_instruction(&mut robot, &arena, &Instruction::Pop(Register::D0), &mut command_queue).unwrap();
        assert_eq!(robot.vm_state.registers.get(Register::D0).unwrap() as i64, 0b1000);
    }

    #[test]
    fn test_combat_execution() {
        let (mut robot, arena, mut command_queue) = setup_test_vm();
        robot.vm_state.registers.set(Register::D1, 45.0).unwrap();
        robot.power = 1.0;
        let instruction = Instruction::Fire(Operand::Register(Register::D1));
        execute_instruction(&mut robot, &arena, &instruction, &mut command_queue).unwrap();
        assert_eq!(command_queue.len(), 2);
        assert!(matches!(command_queue[0], ArenaCommand::SpawnProjectile(_)));
    }

    #[test]
    fn test_unknown_opcode_fault() {
        let (_robot, _arena, _command_queue) = setup_test_vm();
    }
}
