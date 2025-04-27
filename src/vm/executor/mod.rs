// VM Instruction execution: decodes and executes instructions, updates VM state and robot state

// Publicly expose submodules (corrected based on directory listing)
mod arithmetic_ops;
mod bitwise_ops;     // Added
mod combat_ops;
mod component_ops;
mod control_flow_ops;// Changed from control_ops
mod instruction_executor; // Added
mod misc_ops;        // Added
mod register_ops;    // Added
mod stack_ops;       // Added
mod trig_ops;        // Added
pub mod processor;   // <-- Make public
// Removed: branch_ops, control_ops, memory_ops, movement_ops (files don't exist)

// Re-export key components for easier access
// pub use arithmetic_ops::ArithmeticOperations; // Unused
// pub use combat_ops::CombatOperations; // Unused
pub use component_ops::ComponentOperations; // Reinstated for tests

pub use crate::vm::instruction::Instruction;
pub use crate::vm::operand::Operand;
pub use instruction_executor::InstructionExecutor;
// pub use processor::InstructionProcessor; // Removed - Unused here
