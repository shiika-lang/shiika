use super::base::*;

impl<'a, 'b> Parser<'a, 'b> {
    pub (in super) fn parse_stmts(&mut self) -> Result<Vec<ast::Statement>, ParseError> {
        let mut ret = Vec::new();
        while true {
            match self.current_token() {
                Token::Eof => break,
                _ => ret.push(self.parse_expr_stmt()?),
            };
            self.expect_sep();
        }
        Ok(ret)
    }

    pub fn parse_expr_stmt(&mut self) -> Result<ast::Statement, ParseError> {
        Ok(self.parse_expr()?.to_statement())
    }
}
