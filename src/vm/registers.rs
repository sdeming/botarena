// VM Register system: register enum, storage, permissions, and access logic

use super::error::RegisterError;

/// Enum for all VM registers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Register {
    // General purpose data registers
    D0,
    D1,
    D2,
    D3,
    D4,
    D5,
    D6,
    D7,
    D8,
    D9,
    D10,
    D11,
    D12,
    D13,
    D14,
    D15,
    D16,
    D17,
    D18, // Added D10-D18
    // Counter
    C,
    // Special registers
    Result,
    Fault,
    // Memory index register
    Index,
    // State registers (read-only)
    Turn,
    Cycle,
    Rand,
    Health,
    Power,
    Component,
    TurretDirection,
    DriveDirection,
    DriveVelocity,
    PosX,
    PosY,
    ForwardDistance,
    BackwardDistance,
    // Weapon state registers (read-only)
    WeaponPower,     // Current power level for weapons
    WeaponCooldown,  // Cooldown remaining for weapons
    TargetDistance,  // Last detected target distance
    TargetDirection, // Last detected target angle
}

impl Register {
    /// Returns true if the register is writable by the VM program
    pub fn is_writable(&self) -> bool {
        matches!(
            self,
            Register::D0
                | Register::D1
                | Register::D2
                | Register::D3
                | Register::D4
                | Register::D5
                | Register::D6
                | Register::D7
                | Register::D8
                | Register::D9
                | Register::D10 // Added D10-D18
                | Register::D11
                | Register::D12
                | Register::D13
                | Register::D14
                | Register::D15
                | Register::D16
                | Register::D17
                | Register::D18
                | Register::C
                | Register::Result
                | Register::Fault
                | Register::Index // Added Index register
        )
    }

    /// Returns true if the register is read-only (state register)
    pub fn is_readonly(&self) -> bool {
        !self.is_writable()
    }
}

/// Storage for all VM registers
#[derive(Debug, Clone)]
pub struct Registers {
    // All registers as f64 (except @c, which is i64 internally)
    data: [f64; 42], // Increased size from 41 to 42 for Index
}

impl Registers {
    pub fn new() -> Self {
        Registers { data: [0.0; 42] } // Update size
    }

    /// Get the index for a register in the data array
    fn idx(reg: Register) -> usize {
        use Register::*;
        match reg {
            D0 => 0,
            D1 => 1,
            D2 => 2,
            D3 => 3,
            D4 => 4,
            D5 => 5,
            D6 => 6,
            D7 => 7,
            D8 => 8,
            D9 => 9,
            D10 => 10,
            D11 => 11,
            D12 => 12,
            D13 => 13,
            D14 => 14,
            D15 => 15,
            D16 => 16,
            D17 => 17,
            D18 => 18,              // Added D10-D18 indices
            C => 19,                // Shifted C
            Result => 20,           // Shifted Result
            Fault => 21,            // Shifted Fault
            Index => 22,            // New Index register
            Turn => 23,             // Shifted Turn
            Cycle => 24,            // Shifted Cycle
            Rand => 25,             // Shifted Rand
            Health => 26,           // Shifted Health
            Power => 27,            // Shifted Power
            Component => 28,        // Shifted Component
            TurretDirection => 29,  // Shifted TurretDirection
            DriveDirection => 30,   // Shifted DriveDirection
            DriveVelocity => 31,    // Shifted DriveVelocity
            PosX => 32,             // Shifted PosX
            PosY => 33,             // Shifted PosY
            ForwardDistance => 34,  // Shifted ForwardDistance
            BackwardDistance => 35, // Shifted BackwardDistance
            WeaponPower => 36,      // Shifted WeaponPower
            WeaponCooldown => 37,   // Shifted WeaponCooldown
            TargetDistance => 38,   // Shifted TargetDistance
            TargetDirection => 39,  // Shifted TargetAngle
        }
    }

    /// Get the value of a register
    pub fn get(&self, reg: Register) -> Result<f64, RegisterError> {
        let idx = Self::idx(reg);
        self.data
            .get(idx)
            .copied()
            .ok_or(RegisterError::InvalidRegister)
    }

    /// Set the value of a register (enforces write permissions)
    pub fn set(&mut self, reg: Register, value: f64) -> Result<(), RegisterError> {
        if !reg.is_writable() {
            return Err(RegisterError::ReadOnlyRegister);
        }
        self.set_internal(reg, value)
    }

    /// Internal method to set a register value without checking permissions
    /// Used by system code to update read-only registers
    pub(crate) fn set_internal(&mut self, reg: Register, value: f64) -> Result<(), RegisterError> {
        let idx = Self::idx(reg);
        if let Some(slot) = self.data.get_mut(idx) {
            *slot = value;
            Ok(())
        } else {
            Err(RegisterError::InvalidRegister)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_read_write() {
        let mut regs = Registers::new();
        assert!(regs.set(Register::D0, 123.0).is_ok());
        assert_eq!(regs.get(Register::D0).unwrap(), 123.0);
        assert!(regs.set(Register::D9, 456.0).is_ok());
        assert_eq!(regs.get(Register::D9).unwrap(), 456.0);
        assert!(regs.set(Register::D10, 789.0).is_ok()); // Test D10 write
        assert_eq!(regs.get(Register::D10).unwrap(), 789.0); // Test D10 read
        assert!(regs.set(Register::D18, 999.0).is_ok()); // Test D18 write
        assert_eq!(regs.get(Register::D18).unwrap(), 999.0); // Test D18 read

        assert!(regs.set(Register::Turn, 5.0).is_err());
        assert_eq!(regs.get(Register::Turn).unwrap(), 0.0);

        // Test internal set bypasses permissions
        assert!(regs.set_internal(Register::Turn, 5.0).is_ok());
        assert_eq!(regs.get(Register::Turn).unwrap(), 5.0);
    }

    #[test]
    fn test_register_permissions() {
        assert!(Register::D0.is_writable());
        assert!(Register::D9.is_writable());
        assert!(Register::D10.is_writable()); // Test D10 permission
        assert!(Register::D18.is_writable()); // Test D18 permission
        assert!(!Register::Turn.is_writable());
        assert!(Register::Turn.is_readonly());
    }

    #[test]
    fn test_read_only_registers() {
        let mut regs = Registers::new();
        // Check that setting read-only registers via set() fails
        assert_eq!(
            regs.set(Register::Turn, 1.0),
            Err(RegisterError::ReadOnlyRegister)
        );
        assert_eq!(
            regs.set(Register::Cycle, 1.0),
            Err(RegisterError::ReadOnlyRegister)
        );
        assert_eq!(
            regs.set(Register::Rand, 1.0),
            Err(RegisterError::ReadOnlyRegister)
        );
        assert_eq!(
            regs.set(Register::Health, 1.0),
            Err(RegisterError::ReadOnlyRegister)
        );
        assert_eq!(
            regs.set(Register::Power, 1.0),
            Err(RegisterError::ReadOnlyRegister)
        );
        assert_eq!(
            regs.set(Register::Component, 1.0),
            Err(RegisterError::ReadOnlyRegister)
        );
        assert_eq!(
            regs.set(Register::TurretDirection, 1.0),
            Err(RegisterError::ReadOnlyRegister)
        );
        assert_eq!(
            regs.set(Register::DriveDirection, 1.0),
            Err(RegisterError::ReadOnlyRegister)
        );
        assert_eq!(
            regs.set(Register::DriveVelocity, 1.0),
            Err(RegisterError::ReadOnlyRegister)
        );
        assert_eq!(
            regs.set(Register::PosX, 1.0),
            Err(RegisterError::ReadOnlyRegister)
        );
        assert_eq!(
            regs.set(Register::PosY, 1.0),
            Err(RegisterError::ReadOnlyRegister)
        );
        assert_eq!(
            regs.set(Register::ForwardDistance, 1.0),
            Err(RegisterError::ReadOnlyRegister)
        );
        assert_eq!(
            regs.set(Register::BackwardDistance, 1.0),
            Err(RegisterError::ReadOnlyRegister)
        );
        assert_eq!(
            regs.set(Register::WeaponPower, 1.0),
            Err(RegisterError::ReadOnlyRegister)
        );
        assert_eq!(
            regs.set(Register::WeaponCooldown, 1.0),
            Err(RegisterError::ReadOnlyRegister)
        );
        assert_eq!(
            regs.set(Register::TargetDistance, 1.0),
            Err(RegisterError::ReadOnlyRegister)
        );
        assert_eq!(
            regs.set(Register::TargetDirection, 1.0),
            Err(RegisterError::ReadOnlyRegister)
        );
    }

    #[test]
    fn test_internal_set_works() {
        let mut regs = Registers::new();
        for i in 0..39 {
            // Find a register that maps to this index (a bit hacky, assumes contiguous)
            // This is just for testing internal_set, not a robust way to iterate registers
            let reg = match i {
                0 => Register::D0,
                1 => Register::D1,
                2 => Register::D2,
                3 => Register::D3,
                4 => Register::D4,
                5 => Register::D5,
                6 => Register::D6,
                7 => Register::D7,
                8 => Register::D8,
                9 => Register::D9,
                10 => Register::D10,
                11 => Register::D11,
                12 => Register::D12,
                13 => Register::D13, // Added D10-D13
                14 => Register::D14,
                15 => Register::D15,
                16 => Register::D16,
                17 => Register::D17, // Added D14-D17
                18 => Register::D18, // Added D18
                19 => Register::C,
                20 => Register::Result,
                21 => Register::Fault,
                22 => Register::Turn,
                23 => Register::Cycle,
                24 => Register::Rand,
                25 => Register::Health,
                26 => Register::Power,
                27 => Register::Component,
                28 => Register::TurretDirection,
                29 => Register::DriveDirection,
                30 => Register::DriveVelocity,
                31 => Register::PosX,
                32 => Register::PosY,
                33 => Register::ForwardDistance,
                34 => Register::BackwardDistance,
                35 => Register::WeaponPower,
                36 => Register::WeaponCooldown,
                37 => Register::TargetDistance,
                38 => Register::TargetDirection,
                _ => panic!(
                    "Index out of bounds for register mapping in test ({} / 39)",
                    i
                ),
            };
            assert!(regs.set_internal(reg, i as f64).is_ok());
            assert_eq!(regs.get(reg).unwrap(), i as f64);
        }
    }
}
