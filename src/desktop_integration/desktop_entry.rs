use std::collections::HashMap;
use std::fmt;
use crate::error::{AppImageError, AppImageResult};

/// Represents a desktop entry file
#[derive(Debug, Clone)]
pub struct DesktopEntry {
    /// The groups in the desktop entry
    groups: HashMap<String, HashMap<String, String>>,
}

impl DesktopEntry {
    /// Create a new empty desktop entry
    pub fn new() -> Self {
        Self {
            groups: HashMap::new(),
        }
    }

    /// Check if a key exists in the desktop entry
    pub fn exists(&self, key_path: &str) -> bool {
        let (group, key) = self.split_key_path(key_path);
        self.groups.get(group).map_or(false, |group| group.contains_key(key))
    }

    /// Get a value from the desktop entry
    pub fn get(&self, key_path: &str) -> &str {
        let (group, key) = self.split_key_path(key_path);
        self.groups.get(group)
            .and_then(|group| group.get(key))
            .map(|s| s.as_str())
            .unwrap_or_default()
    }

    /// Set a value in the desktop entry
    pub fn set(&mut self, key_path: &str, value: &str) {
        let parts = key_path.split_once('/').unwrap_or((key_path, ""));
        let (group, key) = (parts.0.to_string(), parts.1.to_string());
        self.groups.entry(group)
            .or_insert_with(HashMap::new)
            .insert(key, value.to_string());
    }

    /// Get all key paths in the desktop entry
    pub fn paths(&self) -> Vec<String> {
        let mut paths = Vec::new();
        for (group, keys) in &self.groups {
            for key in keys.keys() {
                paths.push(format!("{}/{}", group, key));
            }
        }
        paths
    }

    /// Split a key path into group and key
    fn split_key_path<'a>(&self, key_path: &'a str) -> (&'a str, &'a str) {
        key_path.split_once('/').unwrap_or((key_path, ""))
    }

    /// Parse a desktop entry from string content
    pub fn parse(content: &str) -> AppImageResult<Self> {
        let mut entry = Self::new();
        let mut current_group = String::new();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if line.starts_with('[') && line.ends_with(']') {
                current_group = line[1..line.len()-1].to_string();
                continue;
            }

            if let Some((key, value)) = line.split_once('=') {
                let key_path = format!("{}/{}", current_group, key.trim());
                entry.set(&key_path, value.trim());
            }
        }

        Ok(entry)
    }
}

impl fmt::Display for DesktopEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (group, entries) in &self.groups {
            writeln!(f, "[{}]", group)?;
            for (key, value) in entries {
                writeln!(f, "{}={}", key, value)?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

/// Represents a desktop entry exec value
#[derive(Debug, Clone)]
pub struct DesktopEntryExecValue {
    /// The command and arguments
    parts: Vec<String>,
}

impl DesktopEntryExecValue {
    /// Parse a desktop entry exec value
    pub fn parse(value: &str) -> AppImageResult<Self> {
        let mut parts = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;
        let mut escape = false;

        for c in value.chars() {
            if escape {
                current.push(c);
                escape = false;
            } else if c == '\\' {
                escape = true;
            } else if c == '"' {
                in_quotes = !in_quotes;
            } else if c.is_whitespace() && !in_quotes {
                if !current.is_empty() {
                    parts.push(current.clone());
                    current.clear();
                }
            } else {
                current.push(c);
            }
        }

        if !current.is_empty() {
            parts.push(current.clone());
        }

        Ok(Self { parts })
    }

    /// Convert to string representation
    pub fn to_string(&self) -> String {
        self.parts.iter()
            .map(|part| {
                if part.contains(' ') || part.contains('"') {
                    format!("\"{}\"", part.replace('"', "\\\""))
                } else {
                    part.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl std::ops::Index<usize> for DesktopEntryExecValue {
    type Output = String;

    fn index(&self, index: usize) -> &Self::Output {
        &self.parts[index]
    }
}

impl std::ops::IndexMut<usize> for DesktopEntryExecValue {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.parts[index]
    }
}

/// Represents a desktop entry strings value
#[derive(Debug, Clone)]
pub struct DesktopEntryStringsValue {
    /// The strings
    strings: Vec<String>,
}

impl DesktopEntryStringsValue {
    /// Parse a desktop entry strings value
    pub fn parse(value: &str) -> AppImageResult<Self> {
        let strings = value.split(';')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        Ok(Self { strings })
    }

    /// Get the strings as a slice
    pub fn iter(&self) -> impl Iterator<Item = &String> {
        self.strings.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_desktop_entry() {
        let mut entry = DesktopEntry::new();
        
        // Test setting and getting values
        entry.set("Desktop Entry/Exec", "test-app");
        assert_eq!(entry.get("Desktop Entry/Exec"), "test-app");
        
        // Test existence check
        assert!(entry.exists("Desktop Entry/Exec"));
        assert!(!entry.exists("Desktop Entry/Icon"));
        
        // Test paths
        let paths = entry.paths();
        assert_eq!(paths.len(), 1);
        assert!(paths.contains(&"Desktop Entry/Exec".to_string()));
    }

    #[test]
    fn test_desktop_entry_exec_value() {
        // Test parsing
        let exec = DesktopEntryExecValue::parse("test-app --arg1 \"arg 2\"").unwrap();
        assert_eq!(exec[0], "test-app");
        assert_eq!(exec[1], "--arg1");
        assert_eq!(exec[2], "arg 2");
        
        // Test string representation
        assert_eq!(exec.to_string(), "test-app --arg1 \"arg 2\"");
    }

    #[test]
    fn test_desktop_entry_strings_value() {
        // Test parsing
        let strings = DesktopEntryStringsValue::parse("action1;action2;action3").unwrap();
        assert_eq!(strings.iter().count(), 3);
        assert!(strings.iter().any(|s| s == "action1"));
        assert!(strings.iter().any(|s| s == "action2"));
        assert!(strings.iter().any(|s| s == "action3"));
    }
} 