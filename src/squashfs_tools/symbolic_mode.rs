use std::fmt;
use std::str::FromStr;
use crate::squashfs_tools::error::{Error, Result};

/// Operation types for symbolic mode changes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SymbolicModeOp {
    Set,   // =
    Add,   // +
    Remove, // -
    Octal, // Octal number
}

/// Represents a single mode operation
#[derive(Debug)]
pub struct ModeData {
    pub operation: SymbolicModeOp,
    pub mode: i32,
    pub mask: u32,
    pub x_flag: bool,
    pub next: Option<Box<ModeData>>,
}

impl ModeData {
    fn new(operation: SymbolicModeOp, mode: i32, mask: u32, x_flag: bool) -> Self {
        Self {
            operation,
            mode,
            mask,
            x_flag,
            next: None,
        }
    }
}

/// Parse an octal mode string
fn parse_octal_mode(s: &str) -> Result<ModeData> {
    let mode = u32::from_str_radix(s, 8)
        .map_err(|_| Error::InvalidMode("Invalid octal mode string".to_string()))?;
    
    if mode > 0o7777 {
        return Err(Error::InvalidMode("Octal mode out of range".to_string()));
    }

    Ok(ModeData::new(
        SymbolicModeOp::Octal,
        mode as i32,
        0o7777,
        false,
    ))
}

/// Parse a symbolic mode string
fn parse_symbolic_mode(s: &str) -> Result<ModeData> {
    let mut chars = s.chars();
    let mut mask = 0u32;
    let mut operation = None;
    let mut mode = 0i32;
    let mut x_flag = false;

    // Parse ownership specifiers (ugoa)
    while let Some(c) = chars.next() {
        match c {
            'u' => mask |= 0o4700,
            'g' => mask |= 0o2070,
            'o' => mask |= 0o0107,
            'a' => mask = 0o7777,
            '+' | '-' | '=' => {
                operation = Some(match c {
                    '+' => SymbolicModeOp::Add,
                    '-' => SymbolicModeOp::Remove,
                    '=' => SymbolicModeOp::Set,
                    _ => unreachable!(),
                });
                break;
            }
            _ => return Err(Error::InvalidMode(format!("Unexpected character: {}", c))),
        }
    }

    // If no ownership specifiers, default to all
    if mask == 0 {
        mask = 0o7777;
    }

    // Parse operation if not already found
    if operation.is_none() {
        if let Some(c) = chars.next() {
            operation = Some(match c {
                '+' => SymbolicModeOp::Add,
                '-' => SymbolicModeOp::Remove,
                '=' => SymbolicModeOp::Set,
                _ => return Err(Error::InvalidMode(format!("Invalid operation: {}", c))),
            });
        } else {
            return Err(Error::InvalidMode("Missing operation".to_string()));
        }
    }

    // Parse permissions
    while let Some(c) = chars.next() {
        match c {
            'r' => mode |= 0o444,
            'w' => mode |= 0o222,
            'x' => mode |= 0o111,
            's' => mode |= 0o6000,
            't' => mode |= 0o1000,
            'X' => x_flag = true,
            '+' | '-' | '=' => return Err(Error::InvalidMode("Unexpected operation character".to_string())),
            _ => return Err(Error::InvalidMode(format!("Invalid permission: {}", c))),
        }
    }

    Ok(ModeData::new(
        operation.unwrap(),
        mode,
        mask,
        x_flag,
    ))
}

/// Parse a mode string that can be either octal or symbolic
pub fn parse_mode(s: &str) -> Result<ModeData> {
    // Try parsing as octal first
    if let Ok(mode) = parse_octal_mode(s) {
        return Ok(mode);
    }

    // If not octal, parse as symbolic
    parse_symbolic_mode(s)
}

/// Execute mode changes on a given mode value
pub fn execute_mode(mode_data: &ModeData, st_mode: i32) -> i32 {
    let mut result = st_mode;
    let mut current = mode_data;

    while let Some(data) = Some(current) {
        let mut mode = if data.mode < 0 {
            // Handle u/g/o references
            match -data.mode as u8 as char {
                'u' => (result >> 6) & 0o7,
                'g' => (result >> 3) & 0o7,
                'o' => result & 0o7,
                _ => unreachable!(),
            }
        } else if data.x_flag && ((result & 0o4000) == 0o4000 || (result & 0o111) != 0) {
            // Handle X flag
            data.mode | (0o111 & data.mask as i32)
        } else {
            data.mode
        };

        mode &= data.mask as i32;

        match data.operation {
            SymbolicModeOp::Octal => {
                result = (result & 0o170000) | mode;
            }
            SymbolicModeOp::Set => {
                result = (result & !(data.mask as i32)) | mode;
            }
            SymbolicModeOp::Add => {
                result |= mode;
            }
            SymbolicModeOp::Remove => {
                result &= !mode;
            }
        }

        current = match &data.next {
            Some(next) => next,
            None => break,
        };
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_octal_mode() {
        let mode = parse_octal_mode("644").unwrap();
        assert_eq!(mode.operation, SymbolicModeOp::Octal);
        assert_eq!(mode.mode, 0o644);
        assert_eq!(mode.mask, 0o7777);
        assert!(!mode.x_flag);

        assert!(parse_octal_mode("9999").is_err()); // Invalid octal
        assert!(parse_octal_mode("8888").is_err()); // Out of range
    }

    #[test]
    fn test_parse_symbolic_mode() {
        let mode = parse_symbolic_mode("u+w").unwrap();
        assert_eq!(mode.operation, SymbolicModeOp::Add);
        assert_eq!(mode.mode, 0o200);
        assert_eq!(mode.mask, 0o4700);
        assert!(!mode.x_flag);

        let mode = parse_symbolic_mode("g-x").unwrap();
        assert_eq!(mode.operation, SymbolicModeOp::Remove);
        assert_eq!(mode.mode, 0o001);
        assert_eq!(mode.mask, 0o2070);
        assert!(!mode.x_flag);

        let mode = parse_symbolic_mode("o=r").unwrap();
        assert_eq!(mode.operation, SymbolicModeOp::Set);
        assert_eq!(mode.mode, 0o400);
        assert_eq!(mode.mask, 0o0107);
        assert!(!mode.x_flag);

        assert!(parse_symbolic_mode("invalid").is_err());
        assert!(parse_symbolic_mode("u+").is_err());
    }

    #[test]
    fn test_execute_mode() {
        let mode = parse_mode("644").unwrap();
        let result = execute_mode(&mode, 0o755);
        assert_eq!(result, 0o644);

        let mode = parse_mode("u+w").unwrap();
        let result = execute_mode(&mode, 0o644);
        assert_eq!(result, 0o744);

        let mode = parse_mode("g-x").unwrap();
        let result = execute_mode(&mode, 0o755);
        assert_eq!(result, 0o745);

        let mode = parse_mode("o=r").unwrap();
        let result = execute_mode(&mode, 0o755);
        assert_eq!(result, 0o754);
    }

    #[test]
    fn test_x_flag() {
        let mode = parse_mode("a+X").unwrap();
        let result = execute_mode(&mode, 0o644);
        assert_eq!(result, 0o644); // No change for non-executable files

        let result = execute_mode(&mode, 0o755);
        assert_eq!(result, 0o755); // No change for already executable files

        let result = execute_mode(&mode, 0o4000);
        assert_eq!(result, 0o4111); // Add execute for directories
    }
} 