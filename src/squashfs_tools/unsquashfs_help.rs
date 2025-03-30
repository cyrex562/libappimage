use std::io::{self, Write};
use std::process;
use regex::Regex;
use crate::squashfs_tools::error::{Error, Result};
use crate::squashfs_tools::print_pager::{exec_pager, wait_to_die, autowrap_print, autowrap_printf};
use crate::squashfs_tools::compressor::DECOMPRESSORS;

/// Constants for help text formatting
const UNSQUASHFS_SYNTAX: &str = "SYNTAX: {} [OPTIONS] FILESYSTEM [files to extract or exclude (with -excludes) or cat (with -cat )]\n\n";
const SQFSCAT_SYNTAX: &str = "SYNTAX: {} [OPTIONS] FILESYSTEM [list of files to cat to stdout]\n\n";

/// Help text sections for unsquashfs
const UNSQUASHFS_SECTIONS: &[&str] = &[
    "extraction", "information", "xattrs", "runtime", "help", "misc",
    "environment", "exit", "extra", "decompressors"
];

/// Help text sections for sqfscat
const SQFSCAT_SECTIONS: &[&str] = &[
    "runtime", "filter", "help", "environment", "exit", "extra",
    "decompressors"
];

/// Help text for unsquashfs
const UNSQUASHFS_TEXT: &[&str] = &[
    "Filesystem extraction (filtering) options:", "\n",
    "\t-d[est] <pathname>\textract to <pathname>, default \"squashfs-root\".  This option also sets the prefix used when listing the filesystem\n",
    "\t-max[-depth] <levels>\tdescend at most <levels> of directories when extracting\n",
    // ... Add all other help text entries here
    "\n", "Decompressors available:", "\n",
    "\t", DECOMPRESSORS, "\n"
];

/// Help text for sqfscat
const SQFSCAT_TEXT: &[&str] = &[
    "Runtime options:", "\n",
    "\t-v[ersion]\t\tprint version, licence and copyright information\n",
    // ... Add all other help text entries here
    "\n", "Decompressors available:", "\n",
    "\t", DECOMPRESSORS, "\n"
];

/// Command line options for unsquashfs
const UNSQUASHFS_OPTIONS: &[&str] = &[
    "", "", "-dest", "-max-depth", "-excludes", "-exclude-list",
    "-extract-file", "-exclude-file", "-match", "-follow-symlinks",
    // ... Add all other options here
];

/// Command line options for sqfscat
const SQFSCAT_OPTIONS: &[&str] = &[
    "", "", "-version", "-processors", "-mem", "-mem-percent", "-offset",
    "-ignore-errors", "-strict-errors", "-no-exit-code",
    // ... Add all other options here
];

/// Arguments for unsquashfs options
const UNSQUASHFS_ARGS: &[&str] = &[
    "", "", "", "", "", "", "<file>", "<file>", "", "", "", "", "",
    "<time>", "", "", "<file>", "", "", "",
    // ... Add all other arguments here
];

/// Arguments for sqfscat options
const SQFSCAT_ARGS: &[&str] = &[
    "", "", "", "<number>", "<size>", "<percent>", "<bytes>", "", "", "",
    // ... Add all other arguments here
];

/// Print help text for all options
pub fn print_help_all(name: &str, syntax: &str, options_text: &[&str]) -> Result<()> {
    let tty = atty::is(atty::Stream::Stdout);
    let cols = if tty {
        get_column_width()?
    } else {
        80
    };

    let mut output: Box<dyn Write> = if tty {
        let (mut pager, pager_pid) = exec_pager()?;
        Box::new(pager)
    } else {
        Box::new(io::stdout())
    };

    autowrap_printf(&mut output, cols, syntax, name)?;

    for text in options_text {
        autowrap_print(&mut output, text, cols)?;
    }

    if tty {
        drop(output);
        wait_to_die(pager_pid)?;
    }

    Ok(())
}

/// Print help text for a specific option
pub fn print_option(prog_name: &str, opt_name: &str, pattern: &str, 
                   options: &[&str], options_args: &[&str], 
                   options_text: &[&str]) -> Result<()> {
    let regex = Regex::new(pattern)?;
    let mut matched = false;
    let cols = get_column_width()?;

    for (i, option) in options.iter().enumerate() {
        if regex.is_match(option) || regex.is_match(options_args[i]) {
            matched = true;
            autowrap_print(&mut io::stdout(), options_text[i], cols)?;
        }
    }

    if !matched {
        autowrap_printf(&mut io::stderr(), cols, 
            "{}: {} {} does not match any {} option\n", 
            prog_name, opt_name, pattern, prog_name)?;
        process::exit(1);
    }

    Ok(())
}

/// Print help text for a specific section
pub fn print_section(prog_name: &str, opt_name: &str, sec_name: &str,
                    sections: &[&str], options_text: &[&str]) -> Result<()> {
    let tty = atty::is(atty::Stream::Stdout);
    let cols = if tty {
        get_column_width()?
    } else {
        80
    };

    let mut output: Box<dyn Write> = if tty {
        let (mut pager, pager_pid) = exec_pager()?;
        Box::new(pager)
    } else {
        Box::new(io::stdout())
    };

    if sec_name == "sections" || sec_name == "h" {
        autowrap_printf(&mut output, cols, 
            "\nUse following section name to print {} help information for that section\n\n", 
            prog_name)?;
        print_section_names(&mut output, "", cols, sections, options_text)?;
        if tty {
            drop(output);
            wait_to_die(pager_pid)?;
        }
        return Ok(());
    }

    // Find section index
    let section_idx = sections.iter()
        .position(|&s| s == sec_name)
        .ok_or_else(|| Error::InvalidSection(sec_name.to_string()))?;

    // Print section content
    let mut current_section = 0;
    for (i, text) in options_text.iter().enumerate() {
        if is_header(i, options_text) {
            current_section += 1;
        }
        if current_section == section_idx + 1 {
            autowrap_print(&mut output, text, cols)?;
        }
    }

    if tty {
        drop(output);
        wait_to_die(pager_pid)?;
    }

    Ok(())
}

/// Helper function to check if a line is a section header
fn is_header(index: usize, options_text: &[&str]) -> bool {
    if let Some(text) = options_text.get(index) {
        text.ends_with(':')
    } else {
        false
    }
}

/// Print section names
fn print_section_names(output: &mut dyn Write, prefix: &str, cols: usize,
                      sections: &[&str], options_text: &[&str]) -> Result<()> {
    autowrap_printf(output, cols, "{}SECTION NAME\t\tSECTION\n", prefix)?;

    let mut section_idx = 0;
    for (i, text) in options_text.iter().enumerate() {
        if is_header(i, options_text) {
            if let Some(section) = sections.get(section_idx) {
                autowrap_printf(output, cols, "{}{}\t\t{}{}\n", 
                    prefix, section, 
                    if section.len() > 7 { "" } else { "\t" },
                    text)?;
                section_idx += 1;
            }
        }
    }

    Ok(())
}

/// Get terminal column width
fn get_column_width() -> Result<usize> {
    use term_size::dimensions;
    Ok(dimensions().map(|(w, _)| w).unwrap_or(80))
}

/// Public interface functions for unsquashfs
pub fn unsquashfs_help_all() -> Result<()> {
    print_help_all("unsquashfs", UNSQUASHFS_SYNTAX, UNSQUASHFS_TEXT)
}

pub fn unsquashfs_section(opt_name: &str, sec_name: &str) -> Result<()> {
    print_section("unsquashfs", opt_name, sec_name, UNSQUASHFS_SECTIONS, UNSQUASHFS_TEXT)
}

pub fn unsquashfs_option(opt_name: &str, pattern: &str) -> Result<()> {
    print_option("unsquashfs", opt_name, pattern, 
                UNSQUASHFS_OPTIONS, UNSQUASHFS_ARGS, UNSQUASHFS_TEXT)
}

pub fn unsquashfs_help(error: bool) -> Result<()> {
    let syntax = if error { UNSQUASHFS_SYNTAX } else { UNSQUASHFS_SYNTAX };
    let mut output: Box<dyn Write> = if error {
        Box::new(io::stderr())
    } else {
        Box::new(io::stdout())
    };
    
    let cols = get_column_width()?;
    autowrap_printf(&mut output, cols, syntax, "unsquashfs")?;
    
    // Print help text and section names
    autowrap_printf(&mut output, cols, 
        "Run\n  \"unsquashfs -help-option <regex>\" to get help on all options matching <regex>\n")?;
    autowrap_printf(&mut output, cols, 
        "\nOr run\n  \"unsquashfs -help-section <section-name>\" to get help on these sections\n")?;
    print_section_names(&mut output, "\t", cols, UNSQUASHFS_SECTIONS, UNSQUASHFS_TEXT)?;
    autowrap_printf(&mut output, cols, 
        "\nOr run\n  \"unsquashfs -help-all\" to get help on all the sections\n")?;

    if error {
        process::exit(1);
    }
    Ok(())
}

/// Public interface functions for sqfscat
pub fn sqfscat_help_all() -> Result<()> {
    print_help_all("sqfscat", SQFSCAT_SYNTAX, SQFSCAT_TEXT)
}

pub fn sqfscat_section(opt_name: &str, sec_name: &str) -> Result<()> {
    print_section("sqfscat", opt_name, sec_name, SQFSCAT_SECTIONS, SQFSCAT_TEXT)
}

pub fn sqfscat_option(opt_name: &str, pattern: &str) -> Result<()> {
    print_option("sqfscat", opt_name, pattern, 
                SQFSCAT_OPTIONS, SQFSCAT_ARGS, SQFSCAT_TEXT)
}

pub fn sqfscat_help(error: bool) -> Result<()> {
    let syntax = if error { SQFSCAT_SYNTAX } else { SQFSCAT_SYNTAX };
    let mut output: Box<dyn Write> = if error {
        Box::new(io::stderr())
    } else {
        Box::new(io::stdout())
    };
    
    let cols = get_column_width()?;
    autowrap_printf(&mut output, cols, syntax, "sqfscat")?;
    
    // Print help text and section names
    autowrap_printf(&mut output, cols, 
        "Run\n  \"sqfscat -help-option <regex>\" to get help on all options matching <regex>\n")?;
    autowrap_printf(&mut output, cols, 
        "\nOr run\n  \"sqfscat -help-section <section-name>\" to get help on these sections\n")?;
    print_section_names(&mut output, "\t", cols, SQFSCAT_SECTIONS, SQFSCAT_TEXT)?;
    autowrap_printf(&mut output, cols, 
        "\nOr run\n  \"sqfscat -help-all\" to get help on all the sections\n")?;

    if error {
        process::exit(1);
    }
    Ok(())
}

/// Display available compressors
pub fn display_compressors() -> Result<()> {
    let cols = get_column_width()?;
    autowrap_print(&mut io::stderr(), &format!("\t{}\n", DECOMPRESSORS), cols)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_header() {
        assert!(is_header(0, UNSQUASHFS_TEXT));
        assert!(!is_header(1, UNSQUASHFS_TEXT));
    }

    #[test]
    fn test_print_help_all() {
        // This test just ensures the function doesn't panic
        print_help_all("test", "SYNTAX: {} [OPTIONS]\n", &["Test help text"]).unwrap();
    }

    #[test]
    fn test_print_option() {
        // This test just ensures the function doesn't panic
        print_option("test", "test", "test", 
                    &["-test"], &["<arg>"], 
                    &["Test option help"]).unwrap();
    }

    #[test]
    fn test_print_section() {
        // This test just ensures the function doesn't panic
        print_section("test", "test", "runtime", 
                     &["runtime"], 
                     &["Runtime options:", "Test option"]).unwrap();
    }
} 