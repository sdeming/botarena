// VM State: registers, stack, ip, fault status, cycle counter, etc.

use super::error::{RegisterError, VMFault};
use super::registers::{Register, Registers};
use super::stack::Stack;
use crate::config;

/// VM state for a robot's program
#[derive(Debug, Clone)]
pub struct VMState {
    pub registers: Registers,
    pub stack: Stack,
    pub ip: usize,                         // Instruction pointer
    pub call_stack: Vec<usize>,            // Call stack to store return addresses
    pub fault: Option<VMFault>,            // Current fault status
    pub turn: u32,                         // Current turn number
    pub cycle: u32,                        // Current cycle within turn
    pub instruction_cycles_remaining: u32, // Cycles left for current instruction
    pub memory: Vec<f64>,                  // Memory array for the VM
}

impl VMState {
    pub fn new() -> Self {
        // Default memory size - can be adjusted as needed
        const DEFAULT_MEMORY_SIZE: usize = 1024;

        VMState {
            registers: Registers::new(),
            stack: Stack::with_size(32), // Use explicit size constructor
            ip: 0,
            call_stack: Vec::with_capacity(config::MAX_CALL_STACK_SIZE),
            fault: None,
            turn: 0,
            cycle: 0,
            instruction_cycles_remaining: 0, // Start ready for first instruction
            memory: vec![0.0; DEFAULT_MEMORY_SIZE], // Initialize memory with zeros
        }
    }

    pub fn advance_ip(&mut self) {
        self.ip += 1;
    }

    pub fn set_fault(&mut self, fault: VMFault) {
        let fault_code = match &fault {
            // Borrow fault here
            VMFault::InvalidInstruction => 1,
            VMFault::InvalidRegister => 2,
            VMFault::PermissionError => 3,
            VMFault::StackOverflow => 4,
            VMFault::StackUnderflow => 5,
            VMFault::DivisionByZero => 6,
            VMFault::NoComponentSelected => 7,
            VMFault::InvalidComponentForOp => 8,
            // Add new fault codes
            VMFault::InsufficientPower => 9,
            VMFault::WeaponOverheated => 10, // Placeholder, not implemented yet
            VMFault::InvalidWeaponPower => 11,
            VMFault::InvalidScanResult => 12, // Placeholder, not implemented yet
            VMFault::ProjectileError => 13,   // Placeholder, not implemented yet
            VMFault::CallStackOverflow => 14,
            VMFault::CallStackUnderflow => 15,
            VMFault::NotImplemented => 99, // Fault code for unimplemented instructions
        };
        self.registers
            .set_internal(Register::Fault, fault_code as f64)
            .unwrap();
        self.fault = Some(fault);
    }

    /// Push a return address onto the call stack
    pub fn push_call_stack(&mut self, return_address: usize) -> Result<(), VMFault> {
        if self.call_stack.len() >= config::MAX_CALL_STACK_SIZE {
            return Err(VMFault::CallStackOverflow);
        }
        self.call_stack.push(return_address);
        Ok(())
    }

    /// Pop a return address from the call stack
    pub fn pop_call_stack(&mut self) -> Result<usize, VMFault> {
        self.call_stack.pop().ok_or(VMFault::CallStackUnderflow)
    }

    /// Internal method for setting the component register
    /// This bypasses the normal register permissions to allow the select instruction to work
    pub(crate) fn set_selected_component(&mut self, component_id: u8) -> Result<(), RegisterError> {
        // Directly access registers to bypass permission check
        self.registers
            .set_internal(Register::Component, component_id as f64)
    }

    // Get memory value at the current index register
    pub fn get_memory_at_index(&mut self) -> Result<f64, VMFault> {
        let index = self
            .registers
            .get(Register::Index)
            .map_err(|_| VMFault::InvalidRegister)?;
        let index = index as usize;

        // Check if index is within bounds
        if index < self.memory.len() {
            Ok(self.memory[index])
        } else {
            Err(VMFault::InvalidRegister) // Reuse existing fault for out-of-bounds memory
        }
    }

    // Store value to memory at the current index register
    pub fn store_memory_at_index(&mut self, value: f64) -> Result<(), VMFault> {
        let index = self
            .registers
            .get(Register::Index)
            .map_err(|_| VMFault::InvalidRegister)?;
        let index = index as usize;

        // Check if index is within bounds
        if index < self.memory.len() {
            self.memory[index] = value;

            // Auto-increment the index register
            let next_index = index as f64 + 1.0;
            self.registers
                .set(Register::Index, next_index)
                .map_err(|_| VMFault::PermissionError)?;

            Ok(())
        } else {
            Err(VMFault::InvalidRegister) // Reuse existing fault for out-of-bounds memory
        }
    }

    // Load memory at current index register into a register and auto-increment
    pub fn load_memory_at_index(&mut self) -> Result<f64, VMFault> {
        let value = self.get_memory_at_index()?;

        // Auto-increment the index register
        let index = self
            .registers
            .get(Register::Index)
            .map_err(|_| VMFault::InvalidRegister)?;
        let next_index = index + 1.0;
        self.registers
            .set(Register::Index, next_index)
            .map_err(|_| VMFault::PermissionError)?;

        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_state_init() {
        let vm = VMState::new();
        assert_eq!(vm.ip, 0);
        assert_eq!(vm.fault, None);
        assert_eq!(vm.turn, 0);
        assert_eq!(vm.cycle, 0);
        assert_eq!(vm.call_stack.len(), 0);
        assert_eq!(vm.call_stack.capacity(), config::MAX_CALL_STACK_SIZE);
    }

    #[test]
    fn test_fault_codes() {
        let mut vm = VMState::new();
        vm.set_fault(VMFault::InvalidInstruction);
        assert_eq!(vm.registers.get(Register::Fault).unwrap(), 1.0);

        vm.set_fault(VMFault::StackOverflow);
        assert_eq!(vm.registers.get(Register::Fault).unwrap(), 4.0);
    }
}
