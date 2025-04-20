// VM Assembly Parser: parses .rasm files, resolves labels/constants, produces instruction list

use super::registers::Register;
use crate::vm::instruction::Instruction;
use crate::vm::operand::Operand;
use std::collections::HashMap;

/// Error type for assembly parsing
#[derive(Debug, Clone)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

/// Result of parsing an assembly program
#[derive(Debug, Clone)]
pub struct ParsedProgram {
    pub instructions: Vec<Instruction>,
    pub labels: HashMap<String, usize>,
}

/// Parse and evaluate a constant expression
/// Supports basic math operations (+, -, *, /, %) and parentheses
/// Example: "ARENA_WIDTH / 2" or "(VALUE_A + VALUE_B) * 1.5"
fn parse_constant_expression(
    expr: &str,
    constants: &HashMap<String, f64>,
    line: usize,
) -> Result<f64, ParseError> {
    // First try to parse as a simple number for backward compatibility
    if let Ok(val) = expr.parse::<f64>() {
        return Ok(val);
    }

    // Try to parse as a constant name
    if let Some(&val) = constants.get(expr) {
        return Ok(val);
    }

    // Simple recursive descent parser for expressions

    // Tokenize the expression - split by operators and parentheses while preserving them
    let expr = expr
        .replace("(", " ( ")
        .replace(")", " ) ")
        .replace("+", " + ")
        .replace("-", " - ")
        .replace("*", " * ")
        .replace("/", " / ")
        .replace("%", " % ");

    let tokens: Vec<&str> = expr.split_whitespace().collect();

    // Define a recursive parsing function
    fn parse_expr<'a>(
        tokens: &'a [&str],
        pos: &mut usize,
        constants: &HashMap<String, f64>,
        line: usize,
    ) -> Result<f64, ParseError> {
        let mut left = parse_term(tokens, pos, constants, line)?;

        while *pos < tokens.len() {
            match tokens[*pos] {
                "+" => {
                    *pos += 1;
                    left += parse_term(tokens, pos, constants, line)?;
                }
                "-" => {
                    *pos += 1;
                    left -= parse_term(tokens, pos, constants, line)?;
                }
                _ => break,
            }
        }

        Ok(left)
    }

    fn parse_term<'a>(
        tokens: &'a [&str],
        pos: &mut usize,
        constants: &HashMap<String, f64>,
        line: usize,
    ) -> Result<f64, ParseError> {
        let mut left = parse_factor(tokens, pos, constants, line)?;

        while *pos < tokens.len() {
            match tokens[*pos] {
                "*" => {
                    *pos += 1;
                    left *= parse_factor(tokens, pos, constants, line)?;
                }
                "/" => {
                    *pos += 1;
                    let right = parse_factor(tokens, pos, constants, line)?;
                    if right == 0.0 {
                        return Err(ParseError {
                            line,
                            message: "Division by zero in constant expression".to_string(),
                        });
                    }
                    left /= right;
                }
                "%" => {
                    *pos += 1;
                    let right = parse_factor(tokens, pos, constants, line)?;
                    if right == 0.0 {
                        return Err(ParseError {
                            line,
                            message: "Modulo by zero in constant expression".to_string(),
                        });
                    }
                    left %= right;
                }
                _ => break,
            }
        }

        Ok(left)
    }

    fn parse_factor<'a>(
        tokens: &'a [&str],
        pos: &mut usize,
        constants: &HashMap<String, f64>,
        line: usize,
    ) -> Result<f64, ParseError> {
        if *pos >= tokens.len() {
            return Err(ParseError {
                line,
                message: "Unexpected end of expression".to_string(),
            });
        }

        let token = tokens[*pos];
        *pos += 1;

        match token {
            "(" => {
                let val = parse_expr(tokens, pos, constants, line)?;
                if *pos < tokens.len() && tokens[*pos] == ")" {
                    *pos += 1;
                    Ok(val)
                } else {
                    Err(ParseError {
                        line,
                        message: "Missing closing parenthesis".to_string(),
                    })
                }
            }
            "-" => {
                // Unary minus
                let val = parse_factor(tokens, pos, constants, line)?;
                Ok(-val)
            }
            _ => {
                // Try parsing as a number
                if let Ok(val) = token.parse::<f64>() {
                    Ok(val)
                } else if let Some(&val) = constants.get(token) {
                    // Try parsing as a constant
                    Ok(val)
                } else {
                    Err(ParseError {
                        line,
                        message: format!("Unknown token in expression: '{}'", token),
                    })
                }
            }
        }
    }

    let mut pos = 0;
    let result = parse_expr(&tokens, &mut pos, constants, line)?;

    if pos < tokens.len() {
        Err(ParseError {
            line,
            message: format!("Unexpected token at end of expression: '{}'", tokens[pos]),
        })
    } else {
        Ok(result)
    }
}

/// Parses a robot assembly program from a string
pub fn parse_assembly(
    source: &str,
    predefined_constants: Option<&HashMap<String, f64>>,
) -> Result<ParsedProgram, ParseError> {
    let mut constants = HashMap::new();
    let mut labels = HashMap::new();

    // Add predefined constants first
    if let Some(predefined) = predefined_constants {
        for (name, value) in predefined {
            constants.insert(name.clone(), *value);
        }
    }

    // First pass: collect user constants and labels, count instructions properly
    let mut line_num = 0;
    let mut instruction_index = 0;
    for line in source.lines() {
        line_num += 1;
        let original_line = line.trim();
        // Skip lines that are empty or *start* with a comment character
        if original_line.is_empty()
            || original_line.starts_with(';')
            || original_line.starts_with('#')
            || original_line.starts_with("//")
        {
            continue;
        }

        // Handle comments potentially anywhere on the line
        let line_no_comment = original_line
            .split(';')
            .next()
            .unwrap() // Take part before first ';'
            .split('#')
            .next()
            .unwrap() // Then take part before first '#'
            .split("//")
            .next()
            .unwrap() // Then take part before first '//'
            .trim(); // Trim remaining whitespace

        if line_no_comment.is_empty() {
            continue;
        }

        if line_no_comment.starts_with(".const") {
            // Parse constant
            let parts: Vec<_> = line_no_comment.split_whitespace().collect();
            if parts.len() >= 3 {
                let name = parts[1].to_string();
                // Check for conflict with predefined constants
                if predefined_constants.map_or(false, |pre| pre.contains_key(&name)) {
                    return Err(ParseError {
                        line: line_num,
                        message: format!("Attempted to redefine built-in constant: {}", name),
                    });
                }

                // Get the expression (everything after the name)
                let expr = line_no_comment.splitn(3, ' ').nth(2).unwrap();

                // Try to evaluate the expression
                match parse_constant_expression(expr, &constants, line_num) {
                    Ok(value) => {
                        if constants.contains_key(&name) {
                            return Err(ParseError {
                                line: line_num,
                                message: format!("Duplicate constant definition: {}", name),
                            });
                        }
                        constants.insert(name, value);
                    }
                    Err(e) => {
                        return Err(ParseError {
                            line: line_num,
                            message: format!(
                                "Invalid expression for constant {}: {}",
                                name, e.message
                            ),
                        });
                    }
                }
            } else {
                return Err(ParseError {
                    line: line_num,
                    message: "Invalid .const format. Use: .const NAME EXPRESSION".to_string(),
                });
            }
            continue; // .const lines don't count as instructions
        }

        let mut is_instruction_line = true;
        if let Some((label_part, rest_part)) = line_no_comment.split_once(':') {
            let label = label_part.trim();
            if !label.is_empty() {
                // Ensure label is not empty
                if labels.contains_key(label) {
                    return Err(ParseError {
                        line: line_num,
                        message: format!("Duplicate label: {}", label),
                    });
                }
                labels.insert(label.to_string(), instruction_index); // Label points to the index of the *next* instruction
            } else {
                return Err(ParseError {
                    line: line_num,
                    message: "Label cannot be empty".to_string(),
                });
            }

            // If the part after ':' is empty or whitespace, it's not an instruction line itself
            if rest_part.trim().is_empty() {
                is_instruction_line = false;
            }
            // If there *is* something after the colon, it counts as an instruction line
        }

        // Increment instruction index only if it's determined to be an instruction line
        if is_instruction_line {
            instruction_index += 1;
        }
    }

    // Second pass: parse instructions using the combined constants map
    line_num = 0;
    let mut collected_results = Vec::new();

    for line in source.lines() {
        line_num += 1;
        let original_line = line.trim();
        // Skip lines that are empty or *start* with a comment character
        if original_line.is_empty()
            || original_line.starts_with(';')
            || original_line.starts_with('#')
            || original_line.starts_with("//")
        {
            continue;
        }
        // Handle comments potentially anywhere on the line
        let line_no_comment = original_line
            .split(';')
            .next()
            .unwrap() // Take part before first ';'
            .split('#')
            .next()
            .unwrap() // Then take part before first '#'
            .split("//")
            .next()
            .unwrap() // Then take part before first '//'
            .trim(); // Trim remaining whitespace

        if line_no_comment.is_empty() {
            continue;
        }

        if line_no_comment.starts_with(".const") {
            continue; // Skip const directives
        }

        // Determine the part of the line containing the potential instruction
        let instruction_part = if let Some((_, rest_part)) = line_no_comment.split_once(':') {
            rest_part.trim() // Instruction is after the colon
        } else {
            line_no_comment // Whole line is the instruction
        };

        // Skip if the instruction part is empty (handles label-only lines)
        if instruction_part.is_empty() {
            continue;
        }

        let parts: Vec<_> = instruction_part.split_whitespace().collect();
        // parts cannot be empty here because instruction_part wasn't empty

        let parse_result: Result<Instruction, ParseError> = match parts[0].to_lowercase().as_str() {
            "push" => {
                if parts.len() > 1 {
                    // Pass the final constants map to parse_operand
                    let op = parse_operand(parts.get(1), &constants, line_num)?;
                    Ok(Instruction::Push(op))
                } else {
                    Err(ParseError {
                        line: line_num,
                        message: "push requires an operand".to_string(),
                    })
                }
            }
            "pop" => {
                if parts.len() == 1 {
                    Ok(Instruction::PopDiscard)
                } else {
                    let reg = parse_register(parts.get(1), line_num)?;
                    Ok(Instruction::Pop(reg))
                }
            }
            "dup" => Ok(Instruction::Dup),
            "swap" => Ok(Instruction::Swap),
            "mov" => {
                if parts.len() > 2 {
                    let dest_reg = parse_register(parts.get(1), line_num)?;
                    let src = parse_operand(parts.get(2), &constants, line_num)?;
                    Ok(Instruction::Mov(dest_reg, src))
                } else {
                    Err(ParseError {
                        line: line_num,
                        message: "mov requires register and operand".to_string(),
                    })
                }
            }
            "lod" => {
                if parts.len() > 1 {
                    let dest_reg = parse_register(parts.get(1), line_num)?;
                    Ok(Instruction::Lod(dest_reg))
                } else {
                    Err(ParseError {
                        line: line_num,
                        message: "lod requires destination register".to_string(),
                    })
                }
            }
            "sto" => {
                if parts.len() > 1 {
                    let value = parse_operand(parts.get(1), &constants, line_num)?;
                    Ok(Instruction::Sto(value))
                } else {
                    Err(ParseError {
                        line: line_num,
                        message: "sto requires a value or register operand".to_string(),
                    })
                }
            }
            "cmp" => {
                if parts.len() > 2 {
                    let left = parse_operand(parts.get(1), &constants, line_num)?;
                    let right = parse_operand(parts.get(2), &constants, line_num)?;
                    Ok(Instruction::Cmp(left, right))
                } else {
                    Err(ParseError {
                        line: line_num,
                        message: "cmp requires two operands".to_string(),
                    })
                }
            }
            "add" => {
                if parts.len() > 2 {
                    // Operand form
                    let left = parse_operand(parts.get(1), &constants, line_num)?;
                    let right = parse_operand(parts.get(2), &constants, line_num)?;
                    Ok(Instruction::AddOp(left, right))
                } else {
                    // Stack form
                    Ok(Instruction::Add)
                }
            },
            "sub" => {
                if parts.len() > 2 {
                    // Operand form
                    let left = parse_operand(parts.get(1), &constants, line_num)?;
                    let right = parse_operand(parts.get(2), &constants, line_num)?;
                    Ok(Instruction::SubOp(left, right))
                } else {
                    // Stack form
                    Ok(Instruction::Sub)
                }
            },
            "mul" => {
                if parts.len() > 2 {
                    // Operand form
                    let left = parse_operand(parts.get(1), &constants, line_num)?;
                    let right = parse_operand(parts.get(2), &constants, line_num)?;
                    Ok(Instruction::MulOp(left, right))
                } else {
                    // Stack form
                    Ok(Instruction::Mul)
                }
            },
            "div" => {
                if parts.len() > 2 {
                    // Operand form
                    let left = parse_operand(parts.get(1), &constants, line_num)?;
                    let right = parse_operand(parts.get(2), &constants, line_num)?;
                    Ok(Instruction::DivOp(left, right))
                } else {
                    // Stack form
                    Ok(Instruction::Div)
                }
            },
            "mod" => {
                if parts.len() > 2 {
                    // Operand form
                    let left = parse_operand(parts.get(1), &constants, line_num)?;
                    let right = parse_operand(parts.get(2), &constants, line_num)?;
                    Ok(Instruction::ModOp(left, right))
                } else {
                    // Stack form
                    Ok(Instruction::Mod)
                }
            },
            "divmod" => Ok(Instruction::Divmod),
            "pow" => {
                if parts.len() > 2 {
                    // Operand form
                    let left = parse_operand(parts.get(1), &constants, line_num)?;
                    let right = parse_operand(parts.get(2), &constants, line_num)?;
                    Ok(Instruction::PowOp(left, right))
                } else {
                    // Stack form
                    Ok(Instruction::Pow)
                }
            },
            "sqrt" => {
                if parts.len() > 1 {
                    // Operand form
                    let op = parse_operand(parts.get(1), &constants, line_num)?;
                    Ok(Instruction::SqrtOp(op))
                } else {
                    // Stack form
                    Ok(Instruction::Sqrt)
                }
            },
            "log" => {
                if parts.len() > 1 {
                    // Operand form
                    let op = parse_operand(parts.get(1), &constants, line_num)?;
                    Ok(Instruction::LogOp(op))
                } else {
                    // Stack form
                    Ok(Instruction::Log)
                }
            },
            "sin" => {
                if parts.len() > 1 {
                    // Operand form
                    let op = parse_operand(parts.get(1), &constants, line_num)?;
                    Ok(Instruction::SinOp(op))
                } else {
                    // Stack form
                    Ok(Instruction::Sin)
                }
            },
            "cos" => {
                if parts.len() > 1 {
                    // Operand form
                    let op = parse_operand(parts.get(1), &constants, line_num)?;
                    Ok(Instruction::CosOp(op))
                } else {
                    // Stack form
                    Ok(Instruction::Cos)
                }
            },
            "tan" => {
                if parts.len() > 1 {
                    // Operand form
                    let op = parse_operand(parts.get(1), &constants, line_num)?;
                    Ok(Instruction::TanOp(op))
                } else {
                    // Stack form
                    Ok(Instruction::Tan)
                }
            },
            "asin" => {
                if parts.len() > 1 {
                    // Operand form
                    let op = parse_operand(parts.get(1), &constants, line_num)?;
                    Ok(Instruction::AsinOp(op))
                } else {
                    // Stack form
                    Ok(Instruction::Asin)
                }
            },
            "acos" => {
                if parts.len() > 1 {
                    // Operand form
                    let op = parse_operand(parts.get(1), &constants, line_num)?;
                    Ok(Instruction::AcosOp(op))
                } else {
                    // Stack form
                    Ok(Instruction::Acos)
                }
            },
            "atan" => {
                if parts.len() > 1 {
                    // Operand form
                    let op = parse_operand(parts.get(1), &constants, line_num)?;
                    Ok(Instruction::AtanOp(op))
                } else {
                    // Stack form
                    Ok(Instruction::Atan)
                }
            },
            "atan2" => {
                if parts.len() > 2 {
                    // Operand form
                    let left = parse_operand(parts.get(1), &constants, line_num)?;
                    let right = parse_operand(parts.get(2), &constants, line_num)?;
                    Ok(Instruction::Atan2Op(left, right))
                } else {
                    // Stack form
                    Ok(Instruction::Atan2)
                }
            },
            "abs" => {
                if parts.len() > 1 {
                    // Operand form
                    let op = parse_operand(parts.get(1), &constants, line_num)?;
                    Ok(Instruction::AbsOp(op))
                } else {
                    // Stack form
                    Ok(Instruction::Abs)
                }
            },
            "and" => {
                if parts.len() > 2 {
                    // Operand form
                    let left = parse_operand(parts.get(1), &constants, line_num)?;
                    let right = parse_operand(parts.get(2), &constants, line_num)?;
                    Ok(Instruction::AndOp(left, right))
                } else {
                    // Stack form
                    Ok(Instruction::And)
                }
            },
            "or" => {
                if parts.len() > 2 {
                    // Operand form
                    let left = parse_operand(parts.get(1), &constants, line_num)?;
                    let right = parse_operand(parts.get(2), &constants, line_num)?;
                    Ok(Instruction::OrOp(left, right))
                } else {
                    // Stack form
                    Ok(Instruction::Or)
                }
            },
            "xor" => {
                if parts.len() > 2 {
                    // Operand form
                    let left = parse_operand(parts.get(1), &constants, line_num)?;
                    let right = parse_operand(parts.get(2), &constants, line_num)?;
                    Ok(Instruction::XorOp(left, right))
                } else {
                    // Stack form
                    Ok(Instruction::Xor)
                }
            },
            "not" => {
                if parts.len() > 1 {
                    // Operand form
                    let op = parse_operand(parts.get(1), &constants, line_num)?;
                    Ok(Instruction::NotOp(op))
                } else {
                    // Stack form
                    Ok(Instruction::Not)
                }
            },
            "shl" => {
                if parts.len() > 2 {
                    // Operand form
                    let left = parse_operand(parts.get(1), &constants, line_num)?;
                    let right = parse_operand(parts.get(2), &constants, line_num)?;
                    Ok(Instruction::ShlOp(left, right))
                } else {
                    // Stack form
                    Ok(Instruction::Shl)
                }
            },
            "shr" => {
                if parts.len() > 2 {
                    // Operand form
                    let left = parse_operand(parts.get(1), &constants, line_num)?;
                    let right = parse_operand(parts.get(2), &constants, line_num)?;
                    Ok(Instruction::ShrOp(left, right))
                } else {
                    // Stack form
                    Ok(Instruction::Shr)
                }
            },
            "jmp" | "jz" | "jnz" | "jl" | "jle" | "jg" | "jge" | "je" | "jne" => {
                let target_label = parts.get(1).ok_or(ParseError {
                    line: line_num,
                    message: "Missing label for jump instruction".to_string(),
                })?;
                // Use .get directly on the borrowed str from parts
                let target = labels
                    .get(*target_label)
                    .copied()
                    .ok_or_else(|| ParseError {
                        line: line_num,
                        message: format!("Unknown label: {}", target_label),
                    })?;
                match parts[0].to_lowercase().as_str() {
                    "jmp" => Ok(Instruction::Jmp(target)),
                    "jz" | "je" => Ok(Instruction::Jz(target)), // je is an alias for jz
                    "jnz" | "jne" => Ok(Instruction::Jnz(target)), // jne is an alias for jnz
                    "jl" => Ok(Instruction::Jl(target)),
                    "jle" => Ok(Instruction::Jle(target)),
                    "jg" => Ok(Instruction::Jg(target)),
                    "jge" => Ok(Instruction::Jge(target)),
                    _ => unreachable!("Jump instruction match failed internally"),
                }
            }
            "call" => {
                let target_label = parts.get(1).ok_or(ParseError {
                    line: line_num,
                    message: "Missing label for call instruction".to_string(),
                })?;
                // Use .get directly on the borrowed str from parts
                let target = labels
                    .get(*target_label)
                    .copied()
                    .ok_or_else(|| ParseError {
                        line: line_num,
                        message: format!("Unknown label: {}", target_label),
                    })?;
                Ok(Instruction::Call(target))
            }
            "ret" => Ok(Instruction::Ret),
            "loop" => {
                let target_label = parts.get(1).ok_or(ParseError {
                    line: line_num,
                    message: "Missing label for loop instruction".to_string(),
                })?;
                // Use .get directly on the borrowed str from parts
                let target = labels
                    .get(*target_label)
                    .copied()
                    .ok_or_else(|| ParseError {
                        line: line_num,
                        message: format!("Unknown label: {}", target_label),
                    })?;
                Ok(Instruction::Loop(target))
            }
            "select" => {
                if parts.len() > 1 {
                    let op = parse_operand(parts.get(1), &constants, line_num)?;
                    Ok(Instruction::Select(op))
                } else {
                    Err(ParseError {
                        line: line_num,
                        message: "select requires component id or register".to_string(),
                    })
                }
            }
            "deselect" => Ok(Instruction::Deselect),
            "rotate" => {
                if parts.len() > 1 {
                    let op = parse_operand(parts.get(1), &constants, line_num)?;
                    Ok(Instruction::Rotate(op))
                } else {
                    Err(ParseError {
                        line: line_num,
                        message: "rotate requires angle operand".to_string(),
                    })
                }
            }
            "drive" => {
                if parts.len() > 1 {
                    let op = parse_operand(parts.get(1), &constants, line_num)?;
                    Ok(Instruction::Drive(op))
                } else {
                    Err(ParseError {
                        line: line_num,
                        message: "drive requires velocity operand".to_string(),
                    })
                }
            }
            "fire" => {
                if parts.len() > 1 {
                    let op = parse_operand(parts.get(1), &constants, line_num)?;
                    Ok(Instruction::Fire(op))
                } else {
                    Err(ParseError {
                        line: line_num,
                        message: "fire requires power operand".to_string(),
                    })
                }
            }
            "scan" => Ok(Instruction::Scan),
            "nop" => Ok(Instruction::Nop),
            "dbg" => {
                if parts.len() > 1 {
                    let op = parse_operand(parts.get(1), &constants, line_num)?;
                    Ok(Instruction::Dbg(op))
                } else {
                    Err(ParseError {
                        line: line_num,
                        message: "dbg requires an operand".to_string(),
                    })
                }
            }
            _ => Err(ParseError {
                line: line_num,
                message: format!("Unknown instruction: {}", parts[0]),
            }),
        };
        collected_results.push(parse_result);
    }

    // Check for any errors during parsing and collect valid instructions
    let instructions: Vec<Instruction> = collected_results.into_iter().collect::<Result<_, _>>()?;

    Ok(ParsedProgram {
        instructions,
        labels,
        // Constants are no longer stored here
    })
}

// Helper: parse an operand (register, value, or constant)
fn parse_operand(
    part: Option<&&str>,
    constants: &HashMap<String, f64>, // Now receives the combined constants
    line: usize,
) -> Result<Operand, ParseError> {
    let s = part.ok_or(ParseError {
        line,
        message: "Missing operand".to_string(),
    })?;

    // Try parsing as number first
    if let Ok(val) = s.parse::<f64>() {
        return Ok(Operand::Value(val));
    }

    // Try parsing as register
    if let Ok(reg) = parse_register(Some(s), line) {
        return Ok(Operand::Register(reg));
    }

    // Try parsing as constant (using the provided map)
    if let Some(&val) = constants.get(*s) {
        return Ok(Operand::Value(val));
    }

    Err(ParseError {
        line,
        message: format!(
            "Invalid operand: {} (not a number, register, or known constant)",
            s
        ),
    })
}

// Helper: parse a register name
fn parse_register(part: Option<&&str>, line: usize) -> Result<Register, ParseError> {
    use Register::*;
    let s = part.ok_or(ParseError {
        line,
        message: "Missing register".to_string(),
    })?;
    match s.to_lowercase().as_str() {
        "@d0" => Ok(D0),
        "@d1" => Ok(D1),
        "@d2" => Ok(D2),
        "@d3" => Ok(D3),
        "@d4" => Ok(D4),
        "@d5" => Ok(D5),
        "@d6" => Ok(D6),
        "@d7" => Ok(D7),
        "@d8" => Ok(D8),
        "@d9" => Ok(D9),
        "@d10" => Ok(D10),
        "@d11" => Ok(D11),
        "@d12" => Ok(D12),
        "@d13" => Ok(D13),
        "@d14" => Ok(D14),
        "@d15" => Ok(D15),
        "@d16" => Ok(D16),
        "@d17" => Ok(D17),
        "@d18" => Ok(D18),
        "@c" => Ok(C),
        "@result" => Ok(Result),
        "@fault" => Ok(Fault),
        "@index" => Ok(Index),
        "@turn" => Ok(Turn),
        "@cycle" => Ok(Cycle),
        "@rand" => Ok(Rand),
        "@health" => Ok(Health),
        "@power" => Ok(Power),
        "@component" => Ok(Component),
        "@turretdirection" | "@turret_direction" => Ok(TurretDirection),
        "@drivedirection" | "@drive_direction" => Ok(DriveDirection),
        "@drivevelocity" | "@drive_velocity" => Ok(DriveVelocity),
        "@posx" | "@pos_x" => Ok(PosX),
        "@posy" | "@pos_y" => Ok(PosY),
        "@forwarddistance" | "@forward_distance" => Ok(ForwardDistance),
        "@backwarddistance" | "@backward_distance" => Ok(BackwardDistance),
        "@weaponpower" | "@weapon_power" => Ok(WeaponPower),
        "@weaponcooldown" | "@weapon_cooldown" => Ok(WeaponCooldown),
        "@targetdistance" | "@target_distance" => Ok(TargetDistance),
        "@targetdirection" | "@target_direction" => Ok(TargetDirection),
        _ => Err(ParseError {
            line,
            message: format!("Unknown register: {}", s),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::instruction::Instruction;
    use crate::vm::operand::Operand;
    use crate::vm::registers::Register;
    // Make sure Register is imported
    use std::collections::HashMap;
    use std::f64::consts::PI;

    #[test]
    fn test_basic_program() {
        let source = r#"
        start:          ; Label definition
            push 1.0    ; Push a value
            pop @d1     ; Pop into register
            mov @d2 5.0 ; Move value to register
            jmp start   ; Jump to label
        "#;
        let result = parse_assembly(source, None);
        assert!(
            result.is_ok(),
            "Basic program test failed: {:?}",
            result.err()
        );
        let program = result.unwrap();
        assert_eq!(program.instructions.len(), 4);
        assert_eq!(*program.labels.get("start").unwrap(), 0);
        assert!(matches!(program.instructions[3], Instruction::Jmp(0)));
    }

    #[test]
    fn test_constant_expression_simple_arithmetic() {
        let source = r#"
        .const SIMPLE_ADD 5 + 3
        .const SIMPLE_SUB 10 - 4 
        .const SIMPLE_MUL 2 * 3
        .const SIMPLE_DIV 10 / 2
        .const SIMPLE_MOD 10 % 3
        push SIMPLE_ADD
        push SIMPLE_SUB
        push SIMPLE_MUL
        push SIMPLE_DIV
        push SIMPLE_MOD
        "#;

        let result = parse_assembly(source, None);
        assert!(
            result.is_ok(),
            "Simple arithmetic test failed: {:?}",
            result.err()
        );
        let program = result.unwrap();

        // Check each instruction has the correct value
        match &program.instructions[0] {
            Instruction::Push(Operand::Value(v)) => assert_eq!(*v, 8.0), // 5 + 3
            _ => panic!("Expected Push instruction with value 8.0"),
        }
        match &program.instructions[1] {
            Instruction::Push(Operand::Value(v)) => assert_eq!(*v, 6.0), // 10 - 4
            _ => panic!("Expected Push instruction with value 6.0"),
        }
        match &program.instructions[2] {
            Instruction::Push(Operand::Value(v)) => assert_eq!(*v, 6.0), // 2 * 3
            _ => panic!("Expected Push instruction with value 6.0"),
        }
        match &program.instructions[3] {
            Instruction::Push(Operand::Value(v)) => assert_eq!(*v, 5.0), // 10 / 2
            _ => panic!("Expected Push instruction with value 5.0"),
        }
        match &program.instructions[4] {
            Instruction::Push(Operand::Value(v)) => assert_eq!(*v, 1.0), // 10 % 3
            _ => panic!("Expected Push instruction with value 1.0"),
        }
    }

    #[test]
    fn test_constant_expression_operator_precedence() {
        let source = r#"
        .const PRECEDENCE_1 2 + 3 * 4       ; Should be 14, not 20
        .const PRECEDENCE_2 (2 + 3) * 4     ; Should be 20
        .const PRECEDENCE_3 10 - 6 / 2      ; Should be 7, not 2
        .const PRECEDENCE_4 (10 - 6) / 2    ; Should be 2
        .const COMPLEX 2 * (3 + 4) - 5 / (2 + 3)  ; Should be 13
        push PRECEDENCE_1
        push PRECEDENCE_2
        push PRECEDENCE_3
        push PRECEDENCE_4
        push COMPLEX
        "#;

        let result = parse_assembly(source, None);
        assert!(
            result.is_ok(),
            "Operator precedence test failed: {:?}",
            result.err()
        );
        let program = result.unwrap();

        match &program.instructions[0] {
            Instruction::Push(Operand::Value(v)) => assert_eq!(*v, 14.0), // 2 + 3 * 4
            _ => panic!("Expected Push instruction with value 14.0"),
        }
        match &program.instructions[1] {
            Instruction::Push(Operand::Value(v)) => assert_eq!(*v, 20.0), // (2 + 3) * 4
            _ => panic!("Expected Push instruction with value 20.0"),
        }
        match &program.instructions[2] {
            Instruction::Push(Operand::Value(v)) => assert_eq!(*v, 7.0), // 10 - 6 / 2
            _ => panic!("Expected Push instruction with value 7.0"),
        }
        match &program.instructions[3] {
            Instruction::Push(Operand::Value(v)) => assert_eq!(*v, 2.0), // (10 - 6) / 2
            _ => panic!("Expected Push instruction with value 2.0"),
        }
        match &program.instructions[4] {
            Instruction::Push(Operand::Value(v)) => assert_eq!(*v, 14.0 - 1.0), // 2 * (3 + 4) - 5 / (2 + 3)
            _ => panic!("Expected Push instruction with value 13.0"),
        }
    }

    #[test]
    fn test_constant_expression_referencing_other_constants() {
        let source = r#"
        .const BASE 10
        .const DERIVED BASE * 2
        .const LEVEL_2 DERIVED + 5
        .const COMBINED BASE + DERIVED + LEVEL_2  ; Should be 10 + 20 + 25 = 55
        push BASE
        push DERIVED
        push LEVEL_2
        push COMBINED
        "#;

        let result = parse_assembly(source, None);
        assert!(
            result.is_ok(),
            "Referencing other constants test failed: {:?}",
            result.err()
        );
        let program = result.unwrap();

        match &program.instructions[0] {
            Instruction::Push(Operand::Value(v)) => assert_eq!(*v, 10.0),
            _ => panic!("Expected Push instruction with value 10.0"),
        }
        match &program.instructions[1] {
            Instruction::Push(Operand::Value(v)) => assert_eq!(*v, 20.0),
            _ => panic!("Expected Push instruction with value 20.0"),
        }
        match &program.instructions[2] {
            Instruction::Push(Operand::Value(v)) => assert_eq!(*v, 25.0),
            _ => panic!("Expected Push instruction with value 25.0"),
        }
        match &program.instructions[3] {
            Instruction::Push(Operand::Value(v)) => assert_eq!(*v, 55.0),
            _ => panic!("Expected Push instruction with value 55.0"),
        }
    }

    #[test]
    fn test_constant_expression_with_predefined_constants() {
        let mut predefined = HashMap::new();
        predefined.insert("ARENA_WIDTH".to_string(), 20.0);
        predefined.insert("ARENA_HEIGHT".to_string(), 15.0);
        predefined.insert("PI".to_string(), PI);

        let source = r#"
        .const CENTER_X ARENA_WIDTH / 2
        .const CENTER_Y ARENA_HEIGHT / 2
        .const AREA ARENA_WIDTH * ARENA_HEIGHT
        push CENTER_X
        push CENTER_Y
        push AREA
        "#;

        let result = parse_assembly(source, Some(&predefined));
        assert!(
            result.is_ok(),
            "Using predefined constants test failed: {:?}",
            result.err()
        );
        let program = result.unwrap();

        match &program.instructions[0] {
            Instruction::Push(Operand::Value(v)) => assert_eq!(*v, 10.0), // 20 / 2
            _ => panic!("Expected Push instruction with value 10.0"),
        }
        match &program.instructions[1] {
            Instruction::Push(Operand::Value(v)) => assert_eq!(*v, 7.5), // 15 / 2
            _ => panic!("Expected Push instruction with value 7.5"),
        }
        match &program.instructions[2] {
            Instruction::Push(Operand::Value(v)) => assert_eq!(*v, 300.0), // 20 * 15
            _ => panic!("Expected Push instruction with value 300.0"),
        }
    }

    #[test]
    fn test_constant_expression_errors() {
        // Test division by zero
        let source1 = ".const DIV_ZERO 5 / 0";
        let result1 = parse_assembly(source1, None);
        assert!(result1.is_err());
        let err1 = result1.err().unwrap();
        assert!(err1.message.contains("Division by zero"));

        // Test modulo by zero
        let source2 = ".const MOD_ZERO 5 % 0";
        let result2 = parse_assembly(source2, None);
        assert!(result2.is_err());
        let err2 = result2.err().unwrap();
        assert!(err2.message.contains("Modulo by zero"));

        // Test undefined constant
        let source3 = ".const UNDEFINED NONEXISTENT + 5";
        let result3 = parse_assembly(source3, None);
        assert!(result3.is_err());
        let err3 = result3.err().unwrap();
        assert!(err3.message.contains("Unknown token"));

        // Test unbalanced parentheses
        let source4 = ".const UNBALANCED (5 + 3 * 2";
        let result4 = parse_assembly(source4, None);
        assert!(result4.is_err());
        let err4 = result4.err().unwrap();
        assert!(err4.message.contains("Missing closing parenthesis"));

        // Test unexpected token
        let source5 = ".const UNEXPECTED 5 + * 3";
        let result5 = parse_assembly(source5, None);
        assert!(result5.is_err());
    }

    #[test]
    fn test_backward_compatibility() {
        // Make sure the old style constants still work
        let source = r#"
        .const OLD_STYLE 42.5
        push OLD_STYLE
        "#;

        let result = parse_assembly(source, None);
        assert!(
            result.is_ok(),
            "Backward compatibility test failed: {:?}",
            result.err()
        );
        let program = result.unwrap();

        match &program.instructions[0] {
            Instruction::Push(Operand::Value(v)) => assert_eq!(*v, 42.5),
            _ => panic!("Expected Push instruction with value 42.5"),
        }
    }

    #[test]
    fn test_labels_and_jumps() {
        let source = r#"
            jmp target
        target: 
            add
            jz target
        "#;
        let result = parse_assembly(source, None);
        assert!(
            result.is_ok(),
            "Labels and jumps test failed: {:?}",
            result.err()
        );
        let program = result.unwrap();
        assert_eq!(program.instructions.len(), 3);
        assert_eq!(*program.labels.get("target").unwrap(), 1); // target label points to 'add'
        assert!(matches!(program.instructions[0], Instruction::Jmp(1)));
        assert!(matches!(program.instructions[1], Instruction::Add));
        assert!(matches!(program.instructions[2], Instruction::Jz(1)));
    }

    #[test]
    fn test_user_constants() {
        let source = ".const MY_VAL 10.5\n push MY_VAL";
        let result = parse_assembly(source, None);
        assert!(
            result.is_ok(),
            "User constants test failed: {:?}",
            result.err()
        );
        let program = result.unwrap();
        assert_eq!(program.instructions.len(), 1);
        assert!(matches!(
            program.instructions[0],
            Instruction::Push(Operand::Value(10.5))
        ));
        // Constants are no longer stored in ParsedProgram
        // assert_eq!(*program.constants.get("MY_VAL").unwrap(), 10.5);
    }

    #[test]
    fn test_predefined_constants() {
        let mut predefined = HashMap::new();
        predefined.insert("ARENA_W".to_string(), 20.0);
        predefined.insert("ARENA_H".to_string(), 15.0);

        let source = "push ARENA_W\nmov @d1 ARENA_H";
        let result = parse_assembly(source, Some(&predefined));
        assert!(
            result.is_ok(),
            "Predefined constants test failed: {:?}",
            result.err()
        );
        let program = result.unwrap();
        assert_eq!(program.instructions.len(), 2);
        assert!(matches!(
            program.instructions[0],
            Instruction::Push(Operand::Value(20.0))
        ));
        assert!(matches!(
            program.instructions[1],
            Instruction::Mov(Register::D1, Operand::Value(15.0))
        ));
    }

    #[test]
    fn test_mixed_constants() {
        let mut predefined = HashMap::new();
        predefined.insert("GRAVITY".to_string(), 9.81);

        let source = ".const SPEED_LIMIT 100.0\npush GRAVITY\npush SPEED_LIMIT";
        let result = parse_assembly(source, Some(&predefined));
        assert!(
            result.is_ok(),
            "Mixed constants test failed: {:?}",
            result.err()
        );
        let program = result.unwrap();
        assert_eq!(program.instructions.len(), 2);
        assert!(matches!(
            program.instructions[0],
            Instruction::Push(Operand::Value(9.81))
        ));
        assert!(matches!(
            program.instructions[1],
            Instruction::Push(Operand::Value(100.0))
        ));
    }

    #[test]
    fn test_redefine_predefined_constant_error() {
        let mut predefined = HashMap::new();
        predefined.insert("BUILT_IN".to_string(), 1.0);

        let source = ".const BUILT_IN 2.0\npush BUILT_IN";
        let result = parse_assembly(source, Some(&predefined));
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(
            err.message
                .contains("Attempted to redefine built-in constant: BUILT_IN")
        );
    }

    #[test]
    fn test_duplicate_user_constant_error() {
        let source = ".const MY_CONST 1.0\n.const MY_CONST 2.0";
        let result = parse_assembly(source, None);
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(
            err.message
                .contains("Duplicate constant definition: MY_CONST")
        );
    }

    #[test]
    fn test_parse_errors() {
        assert!(parse_assembly("invalid_instruction", None).is_err());
        assert!(parse_assembly("push", None).is_err()); // Missing operand
        assert!(parse_assembly("pop @invalid", None).is_err()); // Invalid register
        assert!(parse_assembly("jmp non_existent_label", None).is_err()); // Unknown label
        assert!(parse_assembly("label1:\nlabel1:", None).is_err()); // Duplicate label
        assert!(parse_assembly(".const B", None).is_err()); // Invalid const format
        assert!(parse_assembly(":", None).is_err()); // Empty label
        assert!(parse_assembly("mov @d1", None).is_err()); // Missing operand
        assert!(parse_assembly("cmp @d1", None).is_err()); // Missing operand
    }

    #[test]
    fn test_parse_all_instructions() {
        // More comprehensive test touching most instructions
        let source = r#"
        .const VAL 99.0
        start:
            push 1.0
            push @d1
            pop @d2
            pop
            dup
            swap
            mov @d3 VAL
            mov @index 0
            lod @d4
            sto 42.0
            cmp @d3 100.0
            add 
            sub
            mul
            div 
            mod
            divmod
            and
            or
            xor
            shl
            shr
            jmp start
            jz start
            jnz start
            je start    ; alias for jz
            jne start   ; alias for jnz
            jl start
            jle start
            jg start
            jge start
            select 1
            deselect
            rotate 45.0
            drive 0.5
        "#;
        let result = parse_assembly(source, None);
        assert!(
            result.is_ok(),
            "Parse all instructions failed: {:?}",
            result.err()
        );
        let program = result.unwrap();
        // Count instructions manually: 38 (including divmod, lod, sto)
        assert_eq!(
            program.instructions.len(),
            35,
            "Parsed instruction count mismatch"
        );
    }

    #[test]
    fn test_parse_rotate_register() {
        let source = "rotate @d1";
        let result = parse_assembly(source, None);
        assert!(result.is_ok());
        let program = result.unwrap();
        assert_eq!(program.instructions.len(), 1);
        assert!(matches!(
            program.instructions[0],
            Instruction::Rotate(Operand::Register(Register::D1))
        ));
    }

    #[test]
    fn test_parse_memory_ops() {
        let source = r#"
            mov @index 0  ; Set memory index to 0
            sto 42        ; Store 42 at memory[0] and increment @index
            sto @d1       ; Store value of @d1 at memory[1] and increment @index
            mov @index 0  ; Reset index to 0
            lod @d2       ; Load memory[0] into @d2 and increment @index
            lod @d3       ; Load memory[1] into @d3 and increment @index
        "#;

        let result = parse_assembly(source, None);
        assert!(
            result.is_ok(),
            "Memory ops parsing failed: {:?}",
            result.err()
        );

        let program = result.unwrap();
        assert_eq!(program.instructions.len(), 6, "Should have 6 instructions");

        // Check the instructions
        match &program.instructions[0] {
            Instruction::Mov(Register::Index, Operand::Value(val)) => {
                assert_eq!(*val, 0.0);
            }
            _ => panic!("Expected Mov @index 0 instruction"),
        }

        match &program.instructions[1] {
            Instruction::Sto(Operand::Value(val)) => {
                assert_eq!(*val, 42.0);
            }
            _ => panic!("Expected Sto 42 instruction"),
        }

        match &program.instructions[2] {
            Instruction::Sto(Operand::Register(reg)) => {
                assert_eq!(*reg, Register::D1);
            }
            _ => panic!("Expected Sto @d1 instruction"),
        }

        match &program.instructions[3] {
            Instruction::Mov(Register::Index, Operand::Value(val)) => {
                assert_eq!(*val, 0.0);
            }
            _ => panic!("Expected Mov @index 0 instruction"),
        }

        match &program.instructions[4] {
            Instruction::Lod(reg) => {
                assert_eq!(*reg, Register::D2);
            }
            _ => panic!("Expected Lod @d2 instruction"),
        }

        match &program.instructions[5] {
            Instruction::Lod(reg) => {
                assert_eq!(*reg, Register::D3);
            }
            _ => panic!("Expected Lod @d3 instruction"),
        }
    }

    #[test]
    fn test_parse_arithmetic_stack_ops() {
        // Test parsing of stack-based arithmetic operations
        let source = r#"
            add     ; Stack based add
            sub     ; Stack based subtract
            mul     ; Stack based multiply
            div     ; Stack based divide
            mod     ; Stack based modulo
            divmod  ; Stack based divmod
            pow     ; Stack based power
            sqrt    ; Stack based square root
            log     ; Stack based logarithm
            sin     ; Stack based sine
            cos     ; Stack based cosine
            tan     ; Stack based tangent
            asin    ; Stack based arcsine
            acos    ; Stack based arccosine
            atan    ; Stack based arctangent
            atan2   ; Stack based arctangent2
            abs     ; Stack based absolute value
        "#;

        let result = parse_assembly(source, None);
        assert!(
            result.is_ok(),
            "Parsing stack arithmetic operations failed: {:?}",
            result.err()
        );
        let program = result.unwrap();
        
        // Check all 17 instructions
        assert_eq!(program.instructions.len(), 17, "Expected 17 stack arithmetic instructions");
        
        // Verify each instruction type
        assert!(matches!(program.instructions[0], Instruction::Add));
        assert!(matches!(program.instructions[1], Instruction::Sub));
        assert!(matches!(program.instructions[2], Instruction::Mul));
        assert!(matches!(program.instructions[3], Instruction::Div));
        assert!(matches!(program.instructions[4], Instruction::Mod));
        assert!(matches!(program.instructions[5], Instruction::Divmod));
        assert!(matches!(program.instructions[6], Instruction::Pow));
        assert!(matches!(program.instructions[7], Instruction::Sqrt));
        assert!(matches!(program.instructions[8], Instruction::Log));
        assert!(matches!(program.instructions[9], Instruction::Sin));
        assert!(matches!(program.instructions[10], Instruction::Cos));
        assert!(matches!(program.instructions[11], Instruction::Tan));
        assert!(matches!(program.instructions[12], Instruction::Asin));
        assert!(matches!(program.instructions[13], Instruction::Acos));
        assert!(matches!(program.instructions[14], Instruction::Atan));
        assert!(matches!(program.instructions[15], Instruction::Atan2));
        assert!(matches!(program.instructions[16], Instruction::Abs));
    }

    #[test]
    fn test_parse_arithmetic_operand_ops() {
        // Test parsing of operand-based arithmetic operations
        let source = r#"
            add 5.0 10.0       ; Operand based add
            sub @d0 2.0        ; Operand based subtract
            mul 3.0 @d1        ; Operand based multiply
            div @d2 @d3        ; Operand based divide
            mod 10.0 3.0       ; Operand based modulo
            pow 2.0 3.0        ; Operand based power
            sqrt 16.0          ; Operand based square root
            log 2.718          ; Operand based logarithm
            sin 90.0           ; Operand based sine
            cos 0.0            ; Operand based cosine
            tan 45.0           ; Operand based tangent
            asin 0.5           ; Operand based arcsine
            acos 0.0           ; Operand based arccosine
            atan 1.0           ; Operand based arctangent
            atan2 1.0 1.0      ; Operand based arctangent2
            abs -5.0           ; Operand based absolute value
        "#;

        let result = parse_assembly(source, None);
        assert!(
            result.is_ok(),
            "Parsing operand arithmetic operations failed: {:?}",
            result.err()
        );
        let program = result.unwrap();
        
        // Check all 16 instructions (no operand form for divmod)
        assert_eq!(program.instructions.len(), 16, "Expected 16 operand arithmetic instructions");
        
        // Verify each instruction type and its operands
        match &program.instructions[0] {
            Instruction::AddOp(left, right) => {
                assert!(matches!(left, &Operand::Value(5.0)));
                assert!(matches!(right, &Operand::Value(10.0)));
            },
            _ => panic!("Expected AddOp instruction"),
        }
        
        match &program.instructions[1] {
            Instruction::SubOp(left, right) => {
                assert!(matches!(left, &Operand::Register(Register::D0)));
                assert!(matches!(right, &Operand::Value(2.0)));
            },
            _ => panic!("Expected SubOp instruction"),
        }
        
        match &program.instructions[2] {
            Instruction::MulOp(left, right) => {
                assert!(matches!(left, &Operand::Value(3.0)));
                assert!(matches!(right, &Operand::Register(Register::D1)));
            },
            _ => panic!("Expected MulOp instruction"),
        }
        
        match &program.instructions[3] {
            Instruction::DivOp(left, right) => {
                assert!(matches!(left, &Operand::Register(Register::D2)));
                assert!(matches!(right, &Operand::Register(Register::D3)));
            },
            _ => panic!("Expected DivOp instruction"),
        }
        
        match &program.instructions[4] {
            Instruction::ModOp(left, right) => {
                assert!(matches!(left, &Operand::Value(10.0)));
                assert!(matches!(right, &Operand::Value(3.0)));
            },
            _ => panic!("Expected ModOp instruction"),
        }
        
        match &program.instructions[5] {
            Instruction::PowOp(left, right) => {
                assert!(matches!(left, &Operand::Value(2.0)));
                assert!(matches!(right, &Operand::Value(3.0)));
            },
            _ => panic!("Expected PowOp instruction"),
        }
        
        match &program.instructions[6] {
            Instruction::SqrtOp(op) => {
                assert!(matches!(op, &Operand::Value(16.0)));
            },
            _ => panic!("Expected SqrtOp instruction"),
        }
        
        match &program.instructions[7] {
            Instruction::LogOp(op) => {
                assert!(matches!(op, &Operand::Value(2.718)));
            },
            _ => panic!("Expected LogOp instruction"),
        }
        
        match &program.instructions[8] {
            Instruction::SinOp(op) => {
                assert!(matches!(op, &Operand::Value(90.0)));
            },
            _ => panic!("Expected SinOp instruction"),
        }
        
        match &program.instructions[9] {
            Instruction::CosOp(op) => {
                assert!(matches!(op, &Operand::Value(0.0)));
            },
            _ => panic!("Expected CosOp instruction"),
        }
        
        match &program.instructions[10] {
            Instruction::TanOp(op) => {
                assert!(matches!(op, &Operand::Value(45.0)));
            },
            _ => panic!("Expected TanOp instruction"),
        }
        
        match &program.instructions[11] {
            Instruction::AsinOp(op) => {
                assert!(matches!(op, &Operand::Value(0.5)));
            },
            _ => panic!("Expected AsinOp instruction"),
        }
        
        match &program.instructions[12] {
            Instruction::AcosOp(op) => {
                assert!(matches!(op, &Operand::Value(0.0)));
            },
            _ => panic!("Expected AcosOp instruction"),
        }
        
        match &program.instructions[13] {
            Instruction::AtanOp(op) => {
                assert!(matches!(op, &Operand::Value(1.0)));
            },
            _ => panic!("Expected AtanOp instruction"),
        }
        
        match &program.instructions[14] {
            Instruction::Atan2Op(left, right) => {
                assert!(matches!(left, &Operand::Value(1.0)));
                assert!(matches!(right, &Operand::Value(1.0)));
            },
            _ => panic!("Expected Atan2Op instruction"),
        }
        
        match &program.instructions[15] {
            Instruction::AbsOp(op) => {
                assert!(matches!(op, &Operand::Value(-5.0)));
            },
            _ => panic!("Expected AbsOp instruction"),
        }
    }

    #[test]
    fn test_parse_bitwise_stack_ops() {
        // Test parsing of stack-based bitwise operations
        let source = r#"
            and     ; Stack based AND
            or      ; Stack based OR
            xor     ; Stack based XOR
            not     ; Stack based NOT
            shl     ; Stack based shift left
            shr     ; Stack based shift right
        "#;

        let result = parse_assembly(source, None);
        assert!(
            result.is_ok(),
            "Parsing stack bitwise operations failed: {:?}",
            result.err()
        );
        let program = result.unwrap();
        
        // Check all 6 instructions
        assert_eq!(program.instructions.len(), 6, "Expected 6 stack bitwise instructions");
        
        // Verify each instruction type
        assert!(matches!(program.instructions[0], Instruction::And));
        assert!(matches!(program.instructions[1], Instruction::Or));
        assert!(matches!(program.instructions[2], Instruction::Xor));
        assert!(matches!(program.instructions[3], Instruction::Not));
        assert!(matches!(program.instructions[4], Instruction::Shl));
        assert!(matches!(program.instructions[5], Instruction::Shr));
    }

    #[test]
    fn test_parse_bitwise_operand_ops() {
        // Test parsing of operand-based bitwise operations
        let source = r#"
            and 5 10        ; Operand based AND
            or @d0 2        ; Operand based OR
            xor 3 @d1       ; Operand based XOR
            not 15          ; Operand based NOT
            shl @d2 4       ; Operand based shift left
            shr 16 2        ; Operand based shift right
        "#;

        let result = parse_assembly(source, None);
        assert!(
            result.is_ok(),
            "Parsing operand bitwise operations failed: {:?}",
            result.err()
        );
        let program = result.unwrap();
        
        // Check all 6 instructions
        assert_eq!(program.instructions.len(), 6, "Expected 6 operand bitwise instructions");
        
        // Verify each instruction type and its operands
        match &program.instructions[0] {
            Instruction::AndOp(left, right) => {
                assert!(matches!(left, &Operand::Value(5.0)));
                assert!(matches!(right, &Operand::Value(10.0)));
            },
            _ => panic!("Expected AndOp instruction"),
        }
        
        match &program.instructions[1] {
            Instruction::OrOp(left, right) => {
                assert!(matches!(left, &Operand::Register(Register::D0)));
                assert!(matches!(right, &Operand::Value(2.0)));
            },
            _ => panic!("Expected OrOp instruction"),
        }
        
        match &program.instructions[2] {
            Instruction::XorOp(left, right) => {
                assert!(matches!(left, &Operand::Value(3.0)));
                assert!(matches!(right, &Operand::Register(Register::D1)));
            },
            _ => panic!("Expected XorOp instruction"),
        }
        
        match &program.instructions[3] {
            Instruction::NotOp(op) => {
                assert!(matches!(op, &Operand::Value(15.0)));
            },
            _ => panic!("Expected NotOp instruction"),
        }
        
        match &program.instructions[4] {
            Instruction::ShlOp(left, right) => {
                assert!(matches!(left, &Operand::Register(Register::D2)));
                assert!(matches!(right, &Operand::Value(4.0)));
            },
            _ => panic!("Expected ShlOp instruction"),
        }
        
        match &program.instructions[5] {
            Instruction::ShrOp(left, right) => {
                assert!(matches!(left, &Operand::Value(16.0)));
                assert!(matches!(right, &Operand::Value(2.0)));
            },
            _ => panic!("Expected ShrOp instruction"),
        }
    }

    #[test]
    fn test_parse_control_flow_ops() {
        // Test parsing of control flow operations
        let source = r#"
        start:
            jmp start
            jz start
            jnz start
            je start    ; alias for jz
            jne start   ; alias for jnz
            jl start
            jle start
            jg start
            jge start
            call start
            ret
            loop start
        "#;

        let result = parse_assembly(source, None);
        assert!(
            result.is_ok(),
            "Parsing control flow operations failed: {:?}",
            result.err()
        );
        let program = result.unwrap();
        
        // Check 12 instructions
        assert_eq!(program.instructions.len(), 12, "Expected 12 control flow instructions");
        
        // Verify each instruction type
        assert!(matches!(program.instructions[0], Instruction::Jmp(0)));
        assert!(matches!(program.instructions[1], Instruction::Jz(0)));
        assert!(matches!(program.instructions[2], Instruction::Jnz(0)));
        assert!(matches!(program.instructions[3], Instruction::Jz(0))); // je alias
        assert!(matches!(program.instructions[4], Instruction::Jnz(0))); // jne alias
        assert!(matches!(program.instructions[5], Instruction::Jl(0)));
        assert!(matches!(program.instructions[6], Instruction::Jle(0)));
        assert!(matches!(program.instructions[7], Instruction::Jg(0)));
        assert!(matches!(program.instructions[8], Instruction::Jge(0)));
        assert!(matches!(program.instructions[9], Instruction::Call(0)));
        assert!(matches!(program.instructions[10], Instruction::Ret));
        assert!(matches!(program.instructions[11], Instruction::Loop(0)));
    }

    #[test]
    fn test_parse_component_ops() {
        // Test parsing of component operations
        let source = r#"
            select 1       ; Select component 1 (drive)
            deselect       ; Deselect current component
            rotate 45.0    ; Rotate component
            drive 0.5      ; Set drive velocity
        "#;

        let result = parse_assembly(source, None);
        assert!(
            result.is_ok(),
            "Parsing component operations failed: {:?}",
            result.err()
        );
        let program = result.unwrap();
        
        // Check 4 instructions
        assert_eq!(program.instructions.len(), 4, "Expected 4 component instructions");
        
        // Verify each instruction type and its operands
        match &program.instructions[0] {
            Instruction::Select(op) => {
                assert!(matches!(op, &Operand::Value(1.0)));
            },
            _ => panic!("Expected Select instruction"),
        }
        
        assert!(matches!(program.instructions[1], Instruction::Deselect));
        
        match &program.instructions[2] {
            Instruction::Rotate(op) => {
                assert!(matches!(op, &Operand::Value(45.0)));
            },
            _ => panic!("Expected Rotate instruction"),
        }
        
        match &program.instructions[3] {
            Instruction::Drive(op) => {
                assert!(matches!(op, &Operand::Value(0.5)));
            },
            _ => panic!("Expected Drive instruction"),
        }
    }

    #[test]
    fn test_parse_stack_and_register_ops() {
        // Test parsing of stack and register operations
        let source = r#"
            push 42.0      ; Push value to stack
            push @d0       ; Push register value to stack
            pop @d1        ; Pop from stack to register
            pop            ; Pop and discard
            dup            ; Duplicate top stack value
            swap           ; Swap top two stack values
            mov @d2 10.0   ; Move value to register
            mov @d3 @d4    ; Move register to register
            lod @d5        ; Load from memory to register
            sto 3.14       ; Store value to memory
            sto @d6        ; Store register value to memory
            cmp @d7 @d8    ; Compare registers
        "#;

        let result = parse_assembly(source, None);
        assert!(
            result.is_ok(),
            "Parsing stack and register operations failed: {:?}",
            result.err()
        );
        let program = result.unwrap();
        
        // Check 12 instructions
        assert_eq!(program.instructions.len(), 12, "Expected 12 stack/register instructions");
        
        // Verify each instruction type and its operands
        match &program.instructions[0] {
            Instruction::Push(op) => {
                assert!(matches!(op, &Operand::Value(42.0)));
            },
            _ => panic!("Expected Push instruction with value"),
        }
        
        match &program.instructions[1] {
            Instruction::Push(op) => {
                assert!(matches!(op, &Operand::Register(Register::D0)));
            },
            _ => panic!("Expected Push instruction with register"),
        }
        
        match &program.instructions[2] {
            Instruction::Pop(reg) => {
                assert_eq!(*reg, Register::D1);
            },
            _ => panic!("Expected Pop instruction to register"),
        }
        
        assert!(matches!(program.instructions[3], Instruction::PopDiscard));
        assert!(matches!(program.instructions[4], Instruction::Dup));
        assert!(matches!(program.instructions[5], Instruction::Swap));
        
        match &program.instructions[6] {
            Instruction::Mov(reg, op) => {
                assert_eq!(*reg, Register::D2);
                assert!(matches!(op, &Operand::Value(10.0)));
            },
            _ => panic!("Expected Mov instruction with value"),
        }
        
        match &program.instructions[7] {
            Instruction::Mov(reg, op) => {
                assert_eq!(*reg, Register::D3);
                assert!(matches!(op, &Operand::Register(Register::D4)));
            },
            _ => panic!("Expected Mov instruction with registers"),
        }
        
        match &program.instructions[8] {
            Instruction::Lod(reg) => {
                assert_eq!(*reg, Register::D5);
            },
            _ => panic!("Expected Lod instruction"),
        }
        
        match &program.instructions[9] {
            Instruction::Sto(op) => {
                assert!(matches!(op, &Operand::Value(3.14)));
            },
            _ => panic!("Expected Sto instruction with value"),
        }
        
        match &program.instructions[10] {
            Instruction::Sto(op) => {
                assert!(matches!(op, &Operand::Register(Register::D6)));
            },
            _ => panic!("Expected Sto instruction with register"),
        }
        
        match &program.instructions[11] {
            Instruction::Cmp(left, right) => {
                assert!(matches!(left, &Operand::Register(Register::D7)));
                assert!(matches!(right, &Operand::Register(Register::D8)));
            },
            _ => panic!("Expected Cmp instruction"),
        }
    }

    #[test]
    fn test_parse_misc_ops() {
        // Test parsing of miscellaneous operations
        let source = r#"
            nop            ; No operation
            dbg 123.456    ; Debug value
            dbg @d0        ; Debug register value
        "#;

        let result = parse_assembly(source, None);
        assert!(
            result.is_ok(),
            "Parsing miscellaneous operations failed: {:?}",
            result.err()
        );
        let program = result.unwrap();
        
        // Check 3 instructions
        assert_eq!(program.instructions.len(), 3, "Expected 3 misc instructions");
        
        // Verify each instruction type
        assert!(matches!(program.instructions[0], Instruction::Nop));
        
        match &program.instructions[1] {
            Instruction::Dbg(op) => {
                assert!(matches!(op, &Operand::Value(123.456)));
            },
            _ => panic!("Expected Dbg instruction with value"),
        }
        
        match &program.instructions[2] {
            Instruction::Dbg(op) => {
                assert!(matches!(op, &Operand::Register(Register::D0)));
            },
            _ => panic!("Expected Dbg instruction with register"),
        }
    }
}
