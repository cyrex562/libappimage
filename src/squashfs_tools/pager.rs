use std::io::{self, Write, BufWriter};
use std::process::{Command, Stdio, Child};
use std::fs::File;
use std::path::Path;
use std::os::unix::io::AsRawFd;
use std::sync::Once;
use crate::error::SquashError;

/// Pager types supported by the system
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PagerType {
    Less,
    More,
    Unknown,
}

/// Structure to manage pager state
#[derive(Debug)]
pub struct Pager {
    command: String,
    name: String,
    from_env: bool,
    pager_type: Option<PagerType>,
}

impl Pager {
    /// Create a new pager with default settings
    pub fn new() -> Self {
        Self {
            command: "/usr/bin/pager".to_string(),
            name: "pager".to_string(),
            from_env: false,
            pager_type: None,
        }
    }

    /// Check and set pager from environment variable
    pub fn check_and_set_pager(&mut self, pager: &str) -> Result<(), SquashError> {
        if pager.is_empty() {
            return Err(SquashError::Other("PAGER environment variable is empty!".to_string()));
        }

        // Get base name from path
        let base = Path::new(pager)
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| SquashError::Other("PAGER doesn't have a valid name".to_string()))?;

        // Check for invalid characters
        if pager.contains(|c| c == ' ' || c == '\t') {
            return Err(SquashError::Other("PAGER cannot have spaces or tabs!".to_string()));
        }
        if pager.contains(|c| c == '|' || c == ';') {
            return Err(SquashError::Other("PAGER cannot have pipes or command separators!".to_string()));
        }
        if pager.contains(|c| c == '<' || c == '>' || c == '&') {
            return Err(SquashError::Other("PAGER cannot have file redirections!".to_string()));
        }

        self.command = pager.to_string();
        self.name = base.to_string();
        self.from_env = true;
        Ok(())
    }

    /// Determine the type of pager being used
    fn determine_pager_type(&mut self) -> Result<PagerType, SquashError> {
        let output = Command::new(&self.command)
            .arg("--version")
            .output()
            .map_err(|e| SquashError::Other(format!("Failed to execute pager: {}", e)))?;

        if !output.status.success() {
            return Ok(PagerType::Unknown);
        }

        let version = String::from_utf8_lossy(&output.stdout);
        if version.starts_with("less") {
            Ok(PagerType::Less)
        } else if version.starts_with("more") || version.starts_with("pager") {
            Ok(PagerType::More)
        } else {
            Ok(PagerType::Unknown)
        }
    }

    /// Execute the pager and return a writer
    pub fn exec(&mut self) -> Result<(Box<dyn Write>, Child), SquashError> {
        if self.pager_type.is_none() {
            self.pager_type = Some(self.determine_pager_type()?);
        }

        let mut cmd = Command::new(&self.command);
        
        // Set up command arguments based on pager type
        match self.pager_type {
            Some(PagerType::Less) => cmd.arg("--quit-if-one-screen"),
            Some(PagerType::More) => cmd.arg("--exit-on-eof"),
            _ => &mut cmd,
        };

        // Set up pipe for input
        cmd.stdin(Stdio::piped());

        // Spawn the process
        let mut child = cmd.spawn()
            .map_err(|e| SquashError::Other(format!("Failed to spawn pager: {}", e)))?;

        // Get the stdin pipe
        let stdin = child.stdin.take()
            .ok_or_else(|| SquashError::Other("Failed to get pager stdin".to_string()))?;

        Ok((Box::new(BufWriter::new(stdin)), child))
    }

    /// Wait for the pager process to finish
    pub fn wait_for_child(child: Child) -> Result<(), SquashError> {
        child.wait()
            .map_err(|e| SquashError::Other(format!("Failed to wait for pager: {}", e)))?;
        Ok(())
    }

    /// Get the terminal column width
    pub fn get_column_width() -> Result<usize, SquashError> {
        use libc::{winsize, TIOCGWINSZ, STDOUT_FILENO};
        use std::os::unix::io::AsRawFd;

        let stdout = io::stdout();
        let fd = stdout.as_raw_fd();
        let mut winsize: winsize = unsafe { std::mem::zeroed() };

        unsafe {
            if libc::ioctl(fd, TIOCGWINSZ, &mut winsize) == -1 {
                if libc::isatty(STDOUT_FILENO) != 0 {
                    eprintln!("TIOCGWINSZ ioctl failed, defaulting to 80 columns");
                }
                Ok(80)
            } else {
                Ok(winsize.ws_col as usize)
            }
        }
    }

    /// Print text with automatic wrapping
    pub fn autowrap_print<W: Write>(writer: &mut W, text: &str, maxl: usize) -> Result<(), SquashError> {
        let mut cur = text.chars().peekable();
        let mut tab_out = 0;
        let mut length = 0;

        while let Some(&c) = cur.peek() {
            // Handle indentation
            for _ in 0..tab_out {
                writer.write_all(b"\t")?;
            }

            // Process characters until we hit max length or newline
            while length <= maxl && cur.peek().map_or(false, |&c| c != '\n') {
                match cur.next() {
                    Some('\t') => {
                        tab_out = (length + 8) & !7;
                        length = tab_out;
                    }
                    Some(c) => {
                        length += 1;
                        writer.write_all(c.encode_utf8(&mut [0; 4]).as_bytes())?;
                    }
                    None => break,
                }
            }

            // Handle newlines and wrapping
            if cur.peek() == Some(&'\n') {
                cur.next();
                writer.write_all(b"\n")?;
            } else if length > maxl {
                writer.write_all(b"\n")?;
                while cur.peek() == Some(&' ') {
                    cur.next();
                }
            }
        }

        Ok(())
    }

    /// Print formatted text with automatic wrapping
    pub fn autowrap_printf<W: Write>(writer: &mut W, maxl: usize, fmt: &str, args: std::fmt::Arguments) -> Result<(), SquashError> {
        let text = format!("{}", args);
        Self::autowrap_print(writer, &text, maxl)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_pager_creation() {
        let pager = Pager::new();
        assert_eq!(pager.command, "/usr/bin/pager");
        assert_eq!(pager.name, "pager");
        assert!(!pager.from_env);
    }

    #[test]
    fn test_autowrap_print() {
        let mut output = Vec::new();
        let text = "This is a long line that should wrap at some point because it exceeds the maximum length.";
        Pager::autowrap_print(&mut output, text, 20).unwrap();
        
        let result = String::from_utf8(output).unwrap();
        assert!(result.contains('\n'));
    }

    #[test]
    fn test_column_width() {
        let width = Pager::get_column_width().unwrap();
        assert!(width > 0);
        assert!(width <= 1000); // Reasonable upper limit
    }
} 