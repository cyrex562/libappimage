use std::ffi::CString;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::fs::Metadata;
use std::collections::HashMap;
use regex::Regex;
use nix::unistd::{Uid, Gid};

// Token types for lexical analysis
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Token {
    OpenBracket,
    CloseBracket,
    And,
    Or,
    Not,
    Comma,
    At,
    WhiteSpace,
    String(String),
    Eof,
}

// Expression types
#[derive(Debug)]
pub enum Expr {
    Op {
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        op: Token,
    },
    Atom {
        test: &'static TestEntry,
        args: Vec<String>,
        data: Option<Box<dyn std::any::Any>>,
    },
    Unary {
        expr: Box<Expr>,
        op: Token,
    },
}

// Action types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActionType {
    Fragment,
    Exclude,
    Fragments,
    NoFragments,
    AlwaysFrags,
    NoAlwaysFrags,
    Compressed,
    Uncompressed,
    Uid,
    Gid,
    Guid,
    Mode,
    Empty,
    Move,
    Prune,
    Noop,
    XattrExclude,
    XattrInclude,
    XattrAdd,
}

// File types that actions can operate on
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileType {
    Dir,
    Regular,
    AllLink,
    All,
    Link,
}

// Action logging levels
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActionLogLevel {
    None,
    True,
    False,
    Verbose,
}

// Action data structure
#[derive(Debug)]
pub struct ActionData<'a> {
    pub depth: u32,
    pub name: &'a str,
    pub pathname: String,
    pub subpath: String,
    pub metadata: &'a Metadata,
    pub dir_entry: &'a DirEntry,
    pub root: &'a DirInfo,
}

// Test entry structure
#[derive(Debug)]
pub struct TestEntry {
    pub name: &'static str,
    pub args: i32,
    pub func: fn(&Atom, &ActionData) -> bool,
    pub parse_args: Option<fn(&TestEntry, &Atom) -> bool>,
    pub exclude_ok: bool,
    pub handle_logging: bool,
}

// Atom structure for test operations
#[derive(Debug)]
pub struct Atom {
    pub test: &'static TestEntry,
    pub args: Vec<String>,
    pub data: Option<Box<dyn std::any::Any>>,
}

// Action entry structure
#[derive(Debug)]
pub struct ActionEntry {
    pub name: &'static str,
    pub action_type: ActionType,
    pub args: i32,
    pub file_types: FileType,
    pub parse_args: Option<fn(&ActionEntry, i32, &[String]) -> Option<Box<dyn std::any::Any>>>,
    pub run_action: Option<fn(&Action, &DirEntry)>,
}

// Action structure
#[derive(Debug)]
pub struct Action {
    pub action_type: ActionType,
    pub entry: &'static ActionEntry,
    pub args: Vec<String>,
    pub expr: Expr,
    pub data: Option<Box<dyn std::any::Any>>,
    pub verbose: ActionLogLevel,
}

// Number comparison types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NumberRange {
    Equal,
    Less,
    Greater,
}

// Test number argument structure
#[derive(Debug)]
pub struct TestNumberArg {
    pub size: i64,
    pub range: NumberRange,
}

// Test range argument structure
#[derive(Debug)]
pub struct TestRangeArgs {
    pub start: i64,
    pub end: i64,
}

// Permission operation types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PermOp {
    All,
    Any,
    Exact,
}

// Permission data structure
#[derive(Debug)]
pub struct PermData {
    pub op: PermOp,
    pub mode: u32,
}

// Empty action types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EmptyType {
    All,
    Source,
    Excluded,
}

// Move operation types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MoveOp {
    Rename,
    Move,
}

// Move entry structure
#[derive(Debug)]
pub struct MoveEntry {
    pub ops: Vec<MoveOp>,
    pub dir_entry: Box<DirEntry>,
    pub name: String,
    pub dest: Box<DirInfo>,
    pub next: Option<Box<MoveEntry>>,
}

// Xattr data structure
#[derive(Debug)]
pub struct XattrData {
    pub regex: Regex,
    pub next: Option<Box<XattrData>>,
}

// Error types
#[derive(Debug)]
pub enum ActionError {
    ParseError(String),
    InvalidArgument(String),
    FileError(std::io::Error),
    RegexError(regex::Error),
}

pub type Result<T> = std::result::Result<T, ActionError>;

// Note: DirEntry and DirInfo structures would need to be defined in their respective modules
// These are just placeholder references for now
pub struct DirEntry;
pub struct DirInfo;

// UID info structure
#[derive(Debug)]
pub struct UidInfo {
    pub uid: u32,
}

// GID info structure
#[derive(Debug)]
pub struct GidInfo {
    pub gid: u32,
} 