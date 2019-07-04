use super::base::*;

impl<'a, 'b> Parser<'a, 'b> {
    pub (in super) fn parse_stmts(&mut self) -> Result<Vec<ast::Statement>, Error> {
        let mut ret = Vec::new();
        loop {
            match self.current_token() {
                Token::Eof => break,
                _ => ret.push(self.parse_expr_stmt()?),
            };
            self.expect_sep()?;
        }
        Ok(ret)
    }

    pub fn parse_expr_stmt(&mut self) -> Result<ast::Statement, Error> {
        Ok(self.parse_expr()?.to_statement())
    }
}
