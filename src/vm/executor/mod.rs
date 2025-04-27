// VM Instruction execution: decodes and executes instructions, updates VM state and robot state

// Publicly expose submodules (corrected based on directory listing)
mod arithmetic_ops;
mod bitwise_ops; // Added
mod combat_ops;
pub mod component_ops;
mod control_flow_ops;
mod instruction_executor;
mod misc_ops;
pub mod processor;
mod register_ops;
mod stack_ops;
mod trig_ops;

pub use crate::vm::instruction::Instruction;
pub use crate::vm::operand::Operand;
pub use instruction_executor::InstructionExecutor;
// pub use processor::InstructionProcessor; // Removed - Unused here
