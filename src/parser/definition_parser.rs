use super::Parser;
use super::base::*;
use super::super::ast;

impl<'a, 'b> Parser<'a, 'b> {
    pub fn parse_definitions(&mut self) -> Result<Vec<ast::Definition>, Error> {
        let mut defs = vec![];
        loop {
            if let Some(def) = self.parse_definition()? {
                defs.push(def);
                self.skip_wsn()
            }
            else {
                break
            }
        }
        Ok(defs)
    }

    fn parse_definition(&mut self) -> Result<Option<ast::Definition>, Error> {
        match self.current_token() {
            Token::LowerWord("class") => Ok(Some(self.parse_class_definition()?)),
            Token::LowerWord("def") => Ok(Some(self.parse_method_definition()?)),
            _ => Ok(None),
        }
    }

    fn parse_class_definition(&mut self) -> Result<ast::Definition, Error> {
        let name;
        let defs;

        // `class'
        assert_eq!(*self.current_token(), Token::LowerWord("class"));
        self.consume_token();
        self.skip_ws();

        // Class name
        match self.current_token() {
            Token::UpperWord(s) => { name = s.to_string(); self.consume_token(); },
            token => return Err(parse_error!(self, "class name must start with A-Z but got {:?}", token))
        }
        self.expect_sep()?;

        // Internal definitions
        defs = self.parse_definitions()?;

        // `end'
        match self.current_token() {
            Token::LowerWord("end") => { self.consume_token(); },
            token => return Err(parse_error!(self, "missing `end' for class {:?}; got {:?}", name, token))
        }
        
        Ok(ast::Definition::ClassDefinition { name, defs })
    }

    pub fn parse_method_definition(&mut self) -> Result<ast::Definition, Error> {
        let name;
        let params;

        // `def'
        assert_eq!(*self.current_token(), Token::LowerWord("def"));
        self.consume_token();
        self.skip_ws();

        // Method name
        match self.current_token() {
            Token::LowerWord(s) => { name = s.to_string(); self.consume_token(); },
            token => return Err(parse_error!(self, "method name must start with a-z but got {:?}", token))
        }
        self.skip_ws();

        // Params (optional)
        match self.current_token() {
            Token::Symbol("(") => { params = self.parse_params()? },
            // Has no params
            Token::Separator => { params = vec![]; self.consume_token(); },
            token => return Err(parse_error!(self, "unexpected token in method {:?}: {:?}", name, token))
        }

        // Return type (optional)
        // TODO

        // Body (optional)
        self.skip_wsn();
        let body_stmts = self.parse_stmts()?;

        // `end'
        self.skip_wsn();
        match self.current_token() {
            Token::LowerWord("end") => { self.consume_token(); },
            token => return Err(parse_error!(self, "missing `end' of method {:?}; got {:?}", name, token))
        }

        Ok(ast::Definition::InstanceMethodDefinition { name, params, body_stmts })
    }

    fn parse_params(&mut self) -> Result<Vec<ast::Param>, Error> {
        let mut params = vec!();

        assert_eq!(*self.current_token(), Token::Symbol("("));
        self.consume_token();

        loop {
            // Param
            match self.current_token() {
                Token::LowerWord(_) => { params.push(self.parse_param()?) },
                Token::Symbol(")") => { self.consume_token(); break },
                token => return Err(parse_error!(self, "invalid token in method arguments: {:?}", token))
            }
            self.skip_wsn();
            match self.current_token() {
                Token::Symbol(",") => { self.consume_token(); self.skip_wsn(); },
                Token::Symbol(")") => { self.consume_token(); break }
                token => return Err(parse_error!(self, "invalid token in method arguments: {:?}", token))
            }
        }
        Ok(params)
    }

    fn parse_param(&mut self) -> Result<ast::Param, Error> {
        let name;

        // Name
        match self.current_token() {
            Token::LowerWord(s) => { name = s.to_string(); self.consume_token(); },
            token => return Err(parse_error!(self, "invalid token as method param: {:?}", token))
        }
        self.skip_ws();

        // `:'
        self.expect(Token::Symbol(":"))?;
        self.consume_token();
        self.skip_ws();

        // Type
        let ty = self.parse_ty()?;

        Ok(ast::Param { name, ty })
    }

    fn parse_ty(&mut self) -> Result<ast::Ty, Error> {
        match self.current_token() {
            Token::UpperWord(s) => {
                let ty = ast::Ty { name: s.to_string() };
                self.consume_token();
                Ok(ty)
            },
            token => Err(parse_error!(self, "invalid token as type: {:?}", token))
        }
    }
}
