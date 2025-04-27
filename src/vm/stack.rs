// VM Stack: simple fixed-size f64 stack with push/pop/dup/swap operations

use super::error::StackError;
use std::collections::VecDeque;

/// Fixed-size stack for VM operations
#[derive(Debug, Clone)]
pub struct Stack {
    data: VecDeque<f64>,
    max_size: usize,
}

impl Stack {
    /// Creates a new stack with the specified maximum size
    pub fn with_size(max_size: usize) -> Self {
        Stack {
            data: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    /// Pushes a value onto the stack
    pub fn push(&mut self, value: f64) -> Result<(), StackError> {
        if self.data.len() >= self.max_size {
            return Err(StackError::Overflow);
        }
        self.data.push_back(value);
        Ok(())
    }

    /// Pops a value from the stack
    pub fn pop(&mut self) -> Result<f64, StackError> {
        self.data.pop_back().ok_or(StackError::Underflow)
    }

    /// Duplicates the top value on the stack
    pub fn dup(&mut self) -> Result<(), StackError> {
        if self.data.is_empty() {
            return Err(StackError::Underflow);
        }
        if self.data.len() >= self.max_size {
            return Err(StackError::Overflow);
        }
        let value = *self.data.back().unwrap();
        self.data.push_back(value);
        Ok(())
    }

    /// Swaps the top two values on the stack
    pub fn swap(&mut self) -> Result<(), StackError> {
        if self.data.len() < 2 {
            return Err(StackError::Underflow);
        }
        let len = self.data.len();
        self.data.swap(len - 1, len - 2);
        Ok(())
    }

    /// Returns a slice representing the current stack data (top is last element)
    pub fn view(&self) -> &[f64] {
        self.data.as_slices().0 // VecDeque can be non-contiguous, just get the main slice for debug
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stack_push_pop() {
        let mut stack = Stack::with_size(32);
        assert!(stack.push(1.0).is_ok());
        assert!(stack.push(2.0).is_ok());
        assert_eq!(stack.pop().unwrap(), 2.0);
        assert_eq!(stack.pop().unwrap(), 1.0);
        assert!(stack.pop().is_err());
    }

    #[test]
    fn test_stack_overflow_underflow() {
        let mut stack = Stack::with_size(2);
        assert!(stack.push(1.0).is_ok());
        assert!(stack.push(2.0).is_ok());
        assert!(stack.push(3.0).is_err());
        assert_eq!(stack.pop().unwrap(), 2.0);
        assert_eq!(stack.pop().unwrap(), 1.0);
        assert!(stack.pop().is_err());
    }

    #[test]
    fn test_stack_dup_swap() {
        let mut stack = Stack::with_size(3);
        assert!(stack.push(1.0).is_ok());
        assert!(stack.push(2.0).is_ok());
        assert!(stack.swap().is_ok());
        assert_eq!(stack.pop().unwrap(), 1.0);
        assert_eq!(stack.pop().unwrap(), 2.0);

        assert!(stack.push(3.0).is_ok());
        assert!(stack.dup().is_ok());
        assert_eq!(stack.pop().unwrap(), 3.0);
        assert_eq!(stack.pop().unwrap(), 3.0);
    }
}
