// VM Instruction execution: decodes and executes instructions, updates VM state and robot state

pub mod arithmetic_ops;
pub mod bitwise_ops;
pub mod combat_ops;
pub mod component_ops;
pub mod control_flow_ops;
pub mod instruction_executor;
pub mod misc_ops;
pub mod processor;
pub mod register_ops;
pub mod stack_ops;
pub mod trig_ops;

pub use crate::vm::instruction::Instruction;
pub use crate::vm::operand::Operand;
pub use combat_ops::CombatOperations;
pub use component_ops::ComponentOperations;
pub use instruction_executor::InstructionExecutor;
pub use processor::InstructionProcessor;
