use super::Parser;
use super::lexer::*;
use super::base::*;
use super::super::ast;

impl<'a, 'b> Parser<'a, 'b> {
    pub (in super) fn parse_definition(&mut self) -> Result<ast::Expression, ParseError> {
        match self.current_token() {
            Token::Eof => Err(self.parseerror("unexpected EOF")),
            Token::LowerWord("class") => self.parse_class_definition(),
            //Token::LowerWord("module") => self.parse_module_definition(),
            //Token::LowerWord("enum") => self.parse_enum_definition(),
            _ => panic!("TODO")
        }
    }

    fn parse_class_definition(&mut self) -> Result<ast::Expression, ParseError> {
        assert_eq!(*self.current_token(), Token::LowerWord("class"));
        self.consume_token();
        self.skip_ws();

        match self.current_token() {
            Token::UpperWord(s) => {
            },
            token => {
                let msg = format!("class name must start with A-Z but got {:?}", token);
                return Err(self.parseerror(&msg))
            }
        }
        
    }
}
