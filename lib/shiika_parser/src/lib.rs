/// Parser
///
/// Implementaion rules
/// - Call `skip_ws`/`skip_wsn` before calling other `parse_xx`

/// Create ParseError with `format!`
macro_rules! parse_error {
    ( $self:ident, $( $arg:expr ),* ) => ({
        let msg = format!( $( $arg ),* );
        $self.parseerror(&msg)
    })
}

mod ast_builder;
mod base;
mod definition_parser;
mod error;
mod expression_parser;
pub mod lexer;
mod source_file;
use crate::ast_builder::AstBuilder;
pub use crate::error::Error;
use crate::lexer::Lexer;
use crate::lexer::LexerState;
pub use crate::source_file::SourceFile;
use shiika_ast as ast;
use shiika_ast::Token;

pub struct Parser<'a> {
    pub lexer: Lexer<'a>,
    ast: AstBuilder,
    /// For debug print
    pub lv: usize,
}

impl<'a> Parser<'a> {
    pub fn new(file: &'a SourceFile) -> Parser<'a> {
        Parser {
            lexer: Lexer::new(&file.content),
            ast: AstBuilder::new(&file.path),
            lv: 0,
        }
    }

    /// Parse a method signature
    pub fn parse_signature(sig_str: &str) -> Result<ast::AstMethodSignature, Error> {
        let lexer = Lexer::new_with_state(sig_str, LexerState::MethodName);
        let mut parser = Parser {
            lexer,
            ast: AstBuilder::empty(),
            lv: 0,
        };
        let (ast_sig, _) = parser.parse_method_signature()?;
        // Check if entire string is consumed
        parser.expect_eof()?;
        Ok(ast_sig)
    }

    pub fn parse_files(files: &[SourceFile]) -> Result<ast::Program, Error> {
        let mut program = ast::Program::default();
        for file in files {
            let mut parser = Parser::new(file);
            program.append(&mut parser.parse_program()?);
        }
        Ok(program)
    }

    fn parse_program(&mut self) -> Result<ast::Program, Error> {
        self.skip_wsn()?;
        let toplevel_items = self.parse_toplevel_items()?;
        self.expect_eof()?;
        Ok(ast::Program { toplevel_items })
    }

    pub fn expect_eof(&self) -> Result<(), Error> {
        if *self.current_token() != Token::Eof {
            return Err(parse_error!(
                self,
                "unexpected token: {:?}",
                self.current_token()
            ));
        }
        Ok(())
    }

    fn parse_toplevel_items(&mut self) -> Result<Vec<ast::TopLevelItem>, Error> {
        let mut items = vec![];
        let mut base_seen = false;
        loop {
            match self.current_token() {
                Token::KwRequire => {
                    self.skip_require()?;
                }
                Token::KwBase => {
                    self.consume(Token::KwBase)?;
                    self.skip_ws()?;
                    base_seen = true;
                }
                Token::KwClass => {
                    items.push(ast::TopLevelItem::Def(
                        self.parse_class_definition(base_seen)?,
                    ));
                    base_seen = false;
                }
                Token::KwModule => {
                    items.push(ast::TopLevelItem::Def(self.parse_module_definition()?));
                }
                Token::KwEnum => {
                    items.push(ast::TopLevelItem::Def(self.parse_enum_definition()?));
                }
                Token::KwDef => {
                    return Err(parse_error!(
                        self,
                        "you cannot define toplevel method in Shiika"
                    ));
                }
                Token::Eof | Token::KwEnd => break,
                _ => {
                    let expr = self.parse_expr()?;
                    if let Some(constdef) = expr.as_const_def() {
                        items.push(ast::TopLevelItem::Def(constdef));
                    } else {
                        items.push(ast::TopLevelItem::Expr(expr));
                    }
                }
            }
            self.skip_wsn()?;
        }
        Ok(items)
    }

    /// Skip `require "foo"`
    fn skip_require(&mut self) -> Result<(), Error> {
        assert!(self.consume(Token::KwRequire)?);
        self.skip_ws()?;
        match self.current_token() {
            Token::Str(_) | Token::StrWithInterpolation { .. } => self.consume_token()?,
            _ => return Err(parse_error!(self, "unexpected argument for require")),
        };
        Ok(())
    }
}
