use crate::vm::executor::Operand;
use crate::vm::registers::Register;
use crate::vm::state::VMState;

#[derive(Debug, Clone)]
pub enum Instruction {
    // Stack ops
    Push(Operand),
    Pop(Register),
    PopDiscard,
    Dup,
    Swap,
    // Register ops
    Mov(Register, Operand),
    Cmp(Operand, Operand),
    // Memory ops
    Lod(Register),
    Sto(Operand),
    // Math ops (stack-based)
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Divmod,
    Pow,
    Sqrt,
    Log,
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Atan2,
    Abs,
    // Math ops (operand form -> @result)
    AddOp(Operand, Operand),
    SubOp(Operand, Operand),
    MulOp(Operand, Operand),
    DivOp(Operand, Operand),
    ModOp(Operand, Operand),
    PowOp(Operand, Operand),
    SqrtOp(Operand),
    LogOp(Operand),
    SinOp(Operand),
    CosOp(Operand),
    TanOp(Operand),
    AsinOp(Operand),
    AcosOp(Operand),
    AtanOp(Operand),
    Atan2Op(Operand, Operand),
    AbsOp(Operand),
    // Binary ops (stack-based)
    And,
    Or,
    Xor,
    Not,
    Shl,
    Shr,
    // Binary ops (operand form -> @result)
    AndOp(Operand, Operand),
    OrOp(Operand, Operand),
    XorOp(Operand, Operand),
    NotOp(Operand),
    ShlOp(Operand, Operand),
    ShrOp(Operand, Operand),
    // Control flow
    Jmp(usize),
    Jz(usize),
    Jnz(usize),
    Jl(usize),
    Jle(usize),
    Jg(usize),
    Jge(usize),
    Call(usize),
    Ret,
    Loop(usize),
    // Component ops
    Select(Operand),
    Deselect,
    Rotate(Operand),
    Drive(Operand),
    // Combat ops
    Fire(Operand),
    Scan,
    // Misc
    Nop,
    Dbg(Operand),
}

impl Instruction {
    /// Returns the number of simulation cycles this instruction takes to execute.
    pub fn cycle_cost(&self, vm_state: &VMState) -> u32 {
        use crate::vm::executor::Instruction::{
            Abs, Acos, Add, And, Asin, Atan, Atan2, Cos, Deselect, Div, Divmod, Dup, Log,
            Mod, Mul, Nop, Not, Or, PopDiscard, Pow, Ret, Scan, Shl, Shr, Sin, Sqrt, Sub, Swap,
            Tan, Xor,
        };
        use Instruction::*;
        match self {
            // 1 Cycle
            Push(_) | Pop(_) | PopDiscard | Dup | Swap => 1,
            Mov(_, _) | Cmp(_, _) => 1,
            Lod(_) | Sto(_) => 1,
            And | Or | Xor | Not | Shl | Shr => 1,
            Jmp(_) | Jz(_) | Jnz(_) | Jl(_) | Jle(_) | Jg(_) | Jge(_) => 1,
            Select(_) | Deselect | Drive(_) => 1,
            Nop | Dbg(_) => 1,
            Loop(_) => 1,

            // Arithmetic Ops (Stack Form)
            Add | Sub | Mul | Div | Mod | Divmod | Abs => 1,
            Pow | Sqrt | Log => 2,
            Sin | Cos | Tan => 2,
            Asin | Acos | Atan | Atan2 => 2,

            // Arithmetic Ops (Operand Form)
            AddOp(_, _) | SubOp(_, _) | MulOp(_, _) | DivOp(_, _) | ModOp(_, _) => 1,
            AbsOp(_) => 1,
            PowOp(_, _) | SqrtOp(_) | LogOp(_) => 2,
            SinOp(_) | CosOp(_) | TanOp(_) => 2,
            AsinOp(_) | AcosOp(_) | AtanOp(_) | Atan2Op(_, _) => 2,

            // Binary Ops (Operand Form)
            AndOp(_, _) | OrOp(_, _) | XorOp(_, _) | NotOp(_) | ShlOp(_, _) | ShrOp(_, _) => 1,

            // Control Flow / Subroutines
            Call(_) | Ret => 2,

            // Dynamic Cost
            Rotate(op) => {
                match op {
                    Operand::Value(angle) => 1 + (angle.abs() / 45.0).ceil() as u32,
                    Operand::Register(reg) => {
                        // Get value without mutation if possible, else use average
                        if let Ok(angle) = vm_state.registers.get(*reg) {
                            1 + (angle.abs() / 45.0).ceil() as u32
                        } else {
                            2 // Default/average if register read fails (shouldn't happen here)
                        }
                    }
                }
            }

            // 3 Cycles
            Fire(_) => 3,

            // 1 Cycles
            Scan => 1,
        }
    }
}
