use super::action::*;
use std::iter::Peekable;
use std::str::Chars;

pub struct Lexer<'a> {
    input: Peekable<Chars<'a>>,
    current: String,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Lexer {
            input: input.chars().peekable(),
            current: String::new(),
        }
    }

    fn advance(&mut self) -> Option<char> {
        self.input.next()
    }

    fn peek(&mut self) -> Option<&char> {
        self.input.peek()
    }

    fn is_whitespace(c: char) -> bool {
        c.is_whitespace()
    }

    fn is_quote(c: char) -> bool {
        c == '"'
    }

    fn is_escape(c: char) -> bool {
        c == '\\'
    }

    pub fn next_token(&mut self) -> Token {
        self.current.clear();

        // Skip whitespace
        while let Some(&c) = self.peek() {
            if !Self::is_whitespace(c) {
                break;
            }
            self.advance();
        }

        // Get the next character
        let c = match self.advance() {
            Some(c) => c,
            None => return Token::Eof,
        };

        // Handle special tokens
        match c {
            '(' => return Token::OpenBracket,
            ')' => return Token::CloseBracket,
            ',' => return Token::Comma,
            '@' => return Token::At,
            '!' => return Token::Not,
            '&' => {
                if let Some('&') = self.peek() {
                    self.advance();
                    return Token::And;
                }
                return Token::String(c.to_string());
            }
            '|' => {
                if let Some('|') = self.peek() {
                    self.advance();
                    return Token::Or;
                }
                return Token::String(c.to_string());
            }
            '"' => {
                // Handle quoted strings
                while let Some(c) = self.advance() {
                    if Self::is_quote(c) {
                        break;
                    }
                    if Self::is_escape(c) {
                        if let Some(escaped) = self.advance() {
                            self.current.push(escaped);
                            continue;
                        }
                    }
                    self.current.push(c);
                }
                return Token::String(self.current.clone());
            }
            _ => c,
        }

        // Handle identifiers and other tokens
        if c.is_alphabetic() || c == '_' {
            self.current.push(c);
            while let Some(&c) = self.peek() {
                if c.is_alphanumeric() || c == '_' {
                    self.current.push(c);
                    self.advance();
                } else {
                    break;
                }
            }
            return Token::String(self.current.clone());
        }

        // Handle numbers
        if c.is_digit(10) {
            self.current.push(c);
            while let Some(&c) = self.peek() {
                if c.is_digit(10) || c == '.' {
                    self.current.push(c);
                    self.advance();
                } else {
                    break;
                }
            }
            return Token::String(self.current.clone());
        }

        Token::String(c.to_string())
    }
}

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current_token: Token,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut lexer = Lexer::new(input);
        let current_token = lexer.next_token();
        Parser {
            lexer,
            current_token,
        }
    }

    fn next_token(&mut self) {
        self.current_token = self.lexer.next_token();
    }

    fn expect_token(&mut self, expected: Token) -> Result<(), ActionError> {
        if self.current_token == expected {
            self.next_token();
            Ok(())
        } else {
            Err(ActionError::ParseError(format!(
                "Expected {:?}, got {:?}",
                expected, self.current_token
            )))
        }
    }

    pub fn parse_expr(&mut self) -> Result<Expr, ActionError> {
        let mut expr = self.parse_term()?;

        while self.current_token == Token::And || self.current_token == Token::Or {
            let op = self.current_token;
            self.next_token();
            let rhs = self.parse_term()?;
            expr = Expr::Op {
                lhs: Box::new(expr),
                rhs: Box::new(rhs),
                op,
            };
        }

        Ok(expr)
    }

    fn parse_term(&mut self) -> Result<Expr, ActionError> {
        match self.current_token {
            Token::Not => {
                self.next_token();
                let expr = self.parse_factor()?;
                Ok(Expr::Unary {
                    expr: Box::new(expr),
                    op: Token::Not,
                })
            }
            _ => self.parse_factor(),
        }
    }

    fn parse_factor(&mut self) -> Result<Expr, ActionError> {
        match &self.current_token {
            Token::OpenBracket => {
                self.next_token();
                let expr = self.parse_expr()?;
                self.expect_token(Token::CloseBracket)?;
                Ok(expr)
            }
            Token::String(name) => {
                let test = self.lookup_test(name)?;
                self.next_token();
                let args = self.parse_args()?;
                Ok(Expr::Atom {
                    test,
                    args,
                    data: None,
                })
            }
            _ => Err(ActionError::ParseError(format!(
                "Unexpected token: {:?}",
                self.current_token
            ))),
        }
    }

    fn parse_args(&mut self) -> Result<Vec<String>, ActionError> {
        let mut args = Vec::new();

        if self.current_token == Token::OpenBracket {
            self.next_token();
            while self.current_token != Token::CloseBracket {
                if let Token::String(s) = &self.current_token {
                    args.push(s.clone());
                }
                self.next_token();
                if self.current_token == Token::Comma {
                    self.next_token();
                }
            }
            self.next_token();
        }

        Ok(args)
    }

    fn lookup_test(&self, name: &str) -> Result<&'static TestEntry, ActionError> {
        // This would need to be implemented to look up the test in the test table
        // For now, we'll return an error
        Err(ActionError::ParseError(format!("Unknown test: {}", name)))
    }
}

// Test function implementations
pub fn name_test(atom: &Atom, action_data: &ActionData) -> bool {
    // Implement name test using glob pattern matching
    // This would need to be implemented using a glob pattern matching library
    false
}

pub fn pathname_test(atom: &Atom, action_data: &ActionData) -> bool {
    // Implement pathname test using glob pattern matching
    false
}

pub fn subpathname_test(atom: &Atom, action_data: &ActionData) -> bool {
    // Implement subpathname test using glob pattern matching
    false
}

pub fn filesize_test(atom: &Atom, action_data: &ActionData) -> bool {
    if let Some(data) = &atom.data {
        if let Some(number) = data.downcast_ref::<TestNumberArg>() {
            match number.range {
                NumberRange::Equal => action_data.metadata.len() == number.size as u64,
                NumberRange::Less => action_data.metadata.len() < number.size as u64,
                NumberRange::Greater => action_data.metadata.len() > number.size as u64,
            }
        } else {
            false
        }
    } else {
        false
    }
}

// Add more test function implementations as needed... 