use super::action::*;
use super::action_impl::Parser;
use std::collections::HashMap;
use std::fs::{self, Metadata};
use std::path::Path;

pub struct ActionManager {
    fragment_spec: Vec<Action>,
    exclude_spec: Vec<Action>,
    empty_spec: Vec<Action>,
    move_spec: Vec<Action>,
    prune_spec: Vec<Action>,
    xattr_exc_spec: Vec<Action>,
    xattr_inc_spec: Vec<Action>,
    xattr_add_spec: Vec<Action>,
    other_spec: Vec<Action>,
    move_list: Option<Box<MoveEntry>>,
}

impl ActionManager {
    pub fn new() -> Self {
        ActionManager {
            fragment_spec: Vec::new(),
            exclude_spec: Vec::new(),
            empty_spec: Vec::new(),
            move_spec: Vec::new(),
            prune_spec: Vec::new(),
            xattr_exc_spec: Vec::new(),
            xattr_inc_spec: Vec::new(),
            xattr_add_spec: Vec::new(),
            other_spec: Vec::new(),
            move_list: None,
        }
    }

    pub fn parse_action(&mut self, input: &str, verbose: ActionLogLevel) -> Result<(), ActionError> {
        let mut parser = Parser::new(input);
        let expr = parser.parse_expr()?;

        // Parse action name and arguments
        let action_name = match parser.parse_expr()? {
            Expr::Atom { test, args, data } => test.name,
            _ => return Err(ActionError::ParseError("Expected action name".to_string())),
        };

        // Look up action entry
        let action_entry = self.lookup_action(action_name)?;
        
        // Create action
        let action = Action {
            action_type: action_entry.action_type,
            entry: action_entry,
            args: Vec::new(), // Parse arguments
            expr,
            data: None,
            verbose,
        };

        // Add action to appropriate spec list
        match action.action_type {
            ActionType::Fragment => self.fragment_spec.push(action),
            ActionType::Exclude => self.exclude_spec.push(action),
            ActionType::Empty => self.empty_spec.push(action),
            ActionType::Move => self.move_spec.push(action),
            ActionType::Prune => self.prune_spec.push(action),
            ActionType::XattrExclude => self.xattr_exc_spec.push(action),
            ActionType::XattrInclude => self.xattr_inc_spec.push(action),
            ActionType::XattrAdd => self.xattr_add_spec.push(action),
            _ => self.other_spec.push(action),
        }

        Ok(())
    }

    fn lookup_action(&self, name: &str) -> Result<&'static ActionEntry, ActionError> {
        // This would need to be implemented to look up the action in the action table
        // For now, we'll return an error
        Err(ActionError::ParseError(format!("Unknown action: {}", name)))
    }

    pub fn eval_expr(&self, expr: &Expr, action_data: &ActionData) -> bool {
        match expr {
            Expr::Op { lhs, rhs, op } => {
                let lhs_result = self.eval_expr(lhs, action_data);
                match op {
                    Token::And => lhs_result && self.eval_expr(rhs, action_data),
                    Token::Or => lhs_result || self.eval_expr(rhs, action_data),
                    _ => false,
                }
            }
            Expr::Atom { test, args, data } => {
                (test.func)(&Atom {
                    test,
                    args: args.clone(),
                    data: data.clone(),
                }, action_data)
            }
            Expr::Unary { expr, op } => {
                if *op == Token::Not {
                    !self.eval_expr(expr, action_data)
                } else {
                    false
                }
            }
        }
    }

    pub fn eval_actions(&self, root: &DirInfo, dir_entry: &DirEntry) -> Result<(), ActionError> {
        let action_data = ActionData {
            depth: dir_entry.depth,
            name: &dir_entry.name,
            pathname: dir_entry.pathname.clone(),
            subpath: dir_entry.subpath.clone(),
            metadata: &dir_entry.metadata,
            dir_entry,
            root,
        };

        for action in &self.other_spec {
            if self.eval_expr(&action.expr, &action_data) {
                if let Some(run_action) = action.entry.run_action {
                    run_action(action, dir_entry);
                }
            }
        }

        Ok(())
    }

    pub fn eval_exclude_actions(&self, name: &str, pathname: &str, subpath: &str,
                              metadata: &Metadata, depth: u32, dir_entry: &DirEntry) -> bool {
        let action_data = ActionData {
            depth,
            name,
            pathname: pathname.to_string(),
            subpath: subpath.to_string(),
            metadata,
            dir_entry,
            root: dir_entry.root,
        };

        for action in &self.exclude_spec {
            if self.eval_expr(&action.expr, &action_data) {
                return true;
            }
        }

        false
    }

    pub fn eval_empty_actions(&self, root: &DirInfo, dir_entry: &DirEntry) -> Result<bool, ActionError> {
        // Only evaluate empty actions on empty directories
        if dir_entry.dir_count != 0 {
            return Ok(false);
        }

        let action_data = ActionData {
            depth: dir_entry.depth,
            name: &dir_entry.name,
            pathname: dir_entry.pathname.clone(),
            subpath: dir_entry.subpath.clone(),
            metadata: &dir_entry.metadata,
            dir_entry,
            root,
        };

        for action in &self.empty_spec {
            if self.eval_expr(&action.expr, &action_data) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub fn eval_move_actions(&self, root: &DirInfo, dir_entry: &DirEntry) -> Result<(), ActionError> {
        let action_data = ActionData {
            depth: dir_entry.depth,
            name: &dir_entry.name,
            pathname: dir_entry.pathname.clone(),
            subpath: dir_entry.subpath.clone(),
            metadata: &dir_entry.metadata,
            dir_entry,
            root,
        };

        for action in &self.move_spec {
            if self.eval_expr(&action.expr, &action_data) {
                // Handle move action
                // This would need to be implemented to handle the actual move operation
            }
        }

        Ok(())
    }

    pub fn eval_prune_actions(&self, root: &DirInfo, dir_entry: &DirEntry) -> Result<bool, ActionError> {
        let action_data = ActionData {
            depth: dir_entry.depth,
            name: &dir_entry.name,
            pathname: dir_entry.pathname.clone(),
            subpath: dir_entry.subpath.clone(),
            metadata: &dir_entry.metadata,
            dir_entry,
            root,
        };

        for action in &self.prune_spec {
            if self.eval_expr(&action.expr, &action_data) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub fn do_move_actions(&mut self) -> Result<(), ActionError> {
        while let Some(mut move_entry) = self.move_list.take() {
            // Handle move operation
            // This would need to be implemented to handle the actual move operation
            self.move_list = move_entry.next;
        }
        Ok(())
    }

    pub fn read_action_file(&mut self, filename: &str, verbose: ActionLogLevel) -> Result<(), ActionError> {
        let contents = fs::read_to_string(filename)?;
        
        for line in contents.lines() {
            // Skip empty lines and comments
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }

            // Handle line continuation
            let mut line = line.to_string();
            while line.ends_with('\\') {
                line.pop(); // Remove the backslash
                if let Some(next_line) = contents.lines().next() {
                    line.push_str(next_line.trim());
                } else {
                    break;
                }
            }

            self.parse_action(&line, verbose)?;
        }

        Ok(())
    }
}

// Action implementations
pub fn noop_action(_action: &Action, _dir_entry: &DirEntry) {
    // No operation
}

pub fn uid_action(action: &Action, dir_entry: &DirEntry) {
    if let Some(data) = &action.data {
        if let Some(uid_info) = data.downcast_ref::<UidInfo>() {
            dir_entry.metadata.uid = uid_info.uid;
        }
    }
}

pub fn gid_action(action: &Action, dir_entry: &DirEntry) {
    if let Some(data) = &action.data {
        if let Some(gid_info) = data.downcast_ref::<GidInfo>() {
            dir_entry.metadata.gid = gid_info.gid;
        }
    }
}

pub fn mode_action(action: &Action, dir_entry: &DirEntry) {
    if let Some(data) = &action.data {
        if let Some(mode) = data.downcast_ref::<u32>() {
            dir_entry.metadata.mode = *mode;
        }
    }
}

// Add more action implementations as needed... 