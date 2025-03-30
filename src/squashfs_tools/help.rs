use std::io::{self, Write};
use std::process;
use std::env;
use regex::Regex;
use atty;
use super::error::{ErrorState, SquashError};

/// Constants for help text formatting
const MKSQUASHFS_SYNTAX: &str = "SYNTAX: {} source1 source2 ...  FILESYSTEM [OPTIONS] [-e list of exclude dirs/files]\n\n";
const SQFSTAR_SYNTAX: &str = "SYNTAX: {} [OPTIONS] FILESYSTEM [list of exclude dirs/files]\n\n";

/// Help text sections
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HelpSection {
    Compression,
    Build,
    Time,
    Permissions,
    Pseudo,
    Filter,
    Xattrs,
    Runtime,
    Append,
    Actions,
    Tar,
    Expert,
    Help,
    Misc,
    PseudoDefs,
    Symbolic,
    Environment,
    Exit,
    Extra,
}

/// Help text manager
pub struct HelpManager {
    /// Program name
    prog_name: String,
    /// Whether to display help in a pager
    use_pager: bool,
    /// Column width for text wrapping
    column_width: usize,
    /// Error state for handling errors
    error_state: ErrorState,
}

impl HelpManager {
    /// Create a new help manager
    pub fn new(prog_name: &str) -> Self {
        Self {
            prog_name: prog_name.to_string(),
            use_pager: atty::is(atty::Stream::Stdout),
            column_width: get_column_width(),
            error_state: ErrorState::default(),
        }
    }

    /// Print all help text
    pub fn print_help_all(&mut self, syntax: &str, options_text: &[&str]) -> Result<(), SquashError> {
        let mut output = if self.use_pager {
            exec_pager()?
        } else {
            Box::new(io::stdout()) as Box<dyn Write>
        };

        // Print syntax
        writeln!(output, "{}", syntax.replace("{}", &self.prog_name))?;

        // Print options
        for text in options_text {
            self.autowrap_print(&mut output, text)?;
        }

        // Print compressor usage
        self.display_compressor_usage(&mut output)?;

        Ok(())
    }

    /// Print help for a specific option
    pub fn print_option(&mut self, opt_name: &str, pattern: &str, options: &[&str], 
                       options_args: &[&str], options_text: &[&str]) -> Result<(), SquashError> {
        let regex = Regex::new(pattern).map_err(|e| {
            SquashError::Other(format!("Invalid regex pattern: {}", e))
        })?;

        let mut matched = false;
        let mut output = io::stdout();

        for i in 0..options.len() {
            if regex.is_match(options[i]) || regex.is_match(options_args[i]) {
                matched = true;
                self.autowrap_print(&mut output, options_text[i])?;
            }
        }

        if !matched {
            self.error_state.error(&format!("{}: {} {} does not match any {} option", 
                self.prog_name, opt_name, pattern, self.prog_name));
            return Err(SquashError::Other("No matching options found".to_string()));
        }

        Ok(())
    }

    /// Print help for a specific section
    pub fn print_section(&mut self, opt_name: &str, sec_name: &str, 
                        sections: &[&str], options_text: &[&str]) -> Result<(), SquashError> {
        let mut output = if self.use_pager {
            exec_pager()?
        } else {
            Box::new(io::stdout()) as Box<dyn Write>
        };

        if sec_name == "sections" || sec_name == "h" {
            writeln!(output, "\nUse following section name to print {} help information for that section\n", self.prog_name)?;
            self.print_section_names(&mut output, "", sections, options_text)?;
            return Ok(());
        }

        // Find section index
        let section_idx = sections.iter().position(|&s| s == sec_name)
            .ok_or_else(|| SquashError::Other(format!("Section '{}' not found", sec_name)))?;

        // Print section content
        let mut current_section = 0;
        for text in options_text {
            if self.is_header(text) {
                current_section += 1;
            }
            if current_section == section_idx + 1 {
                self.autowrap_print(&mut output, text)?;
            }
        }

        Ok(())
    }

    /// Print section names
    fn print_section_names(&mut self, prefix: &str, sections: &[&str], 
                          options_text: &[&str]) -> Result<(), SquashError> {
        let mut output = io::stdout();
        writeln!(output, "{}SECTION NAME\t\tSECTION", prefix)?;

        let mut section_idx = 0;
        for (i, text) in options_text.iter().enumerate() {
            if self.is_header(text) {
                writeln!(output, "{}{}\t\t{}{}", 
                    prefix, sections[section_idx],
                    if sections[section_idx].len() > 7 { "" } else { "\t" },
                    text)?;
                section_idx += 1;
            }
        }

        Ok(())
    }

    /// Check if text is a section header
    fn is_header(&self, text: &str) -> bool {
        text.ends_with(':')
    }

    /// Print text with auto-wrapping
    fn autowrap_print(&self, output: &mut impl Write, text: &str) -> Result<(), SquashError> {
        // TODO: Implement text wrapping logic
        writeln!(output, "{}", text).map_err(|e| SquashError::IOError(e))
    }

    /// Display compressor usage
    fn display_compressor_usage(&self, output: &mut impl Write) -> Result<(), SquashError> {
        // TODO: Implement compressor usage display
        Ok(())
    }
}

/// Get terminal column width
fn get_column_width() -> usize {
    if let Some((width, _)) = term_size::dimensions() {
        width
    } else {
        80
    }
}

/// Execute pager for output
fn exec_pager() -> Result<Box<dyn Write>, SquashError> {
    let pager = env::var("PAGER").unwrap_or_else(|_| "/usr/bin/pager".to_string());
    // TODO: Implement pager execution
    Ok(Box::new(io::stdout()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_help_manager() {
        let mut manager = HelpManager::new("test_prog");
        assert_eq!(manager.prog_name, "test_prog");
        assert!(manager.column_width > 0);
    }

    #[test]
    fn test_section_header() {
        let manager = HelpManager::new("test");
        assert!(manager.is_header("Test section:"));
        assert!(!manager.is_header("Not a header"));
    }

    #[test]
    fn test_print_option() {
        let mut manager = HelpManager::new("test");
        let options = &["-test", "-other"];
        let args = &["<arg>", ""];
        let text = &["Test option", "Other option"];

        // Test matching option
        assert!(manager.print_option("test", "-test", options, args, text).is_ok());

        // Test non-matching option
        assert!(manager.print_option("test", "-nonexistent", options, args, text).is_err());
    }
} 