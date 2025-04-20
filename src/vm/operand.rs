use crate::vm::error::VMFault;
use crate::vm::registers::Register;
use crate::vm::state::VMState;

/// Represents a value or register operand
#[derive(Debug, Clone)]
pub enum Operand {
    Value(f64),
    Register(Register),
}

impl Operand {
    /// Gets the operand value using an immutable reference when possible
    /// This should be used when just reading register values
    pub(crate) fn get_value(&self, vm: &VMState) -> Result<f64, VMFault> {
        match self {
            Operand::Value(val) => Ok(*val),
            Operand::Register(r) => {
                let val = vm.registers.get(*r).map_err(|_| VMFault::InvalidRegister)?;
                log::debug!(target: "instructions", "Read register {:?} = {}", r, val);
                Ok(val)
            }
        }
    }

    /// Gets the operand value with a mutable reference to VMState
    /// This should be used only when the operation might need to modify VM state
    pub(crate) fn get_value_mut(&self, vm: &mut VMState) -> Result<f64, VMFault> {
        match self {
            Operand::Value(val) => Ok(*val),
            Operand::Register(r) => {
                let val = vm.registers.get(*r).map_err(|_| VMFault::InvalidRegister)?;
                log::debug!(target: "instructions", "Read register {:?} = {}", r, val);
                Ok(val)
            }
        }
    }
}
