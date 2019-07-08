use super::Parser; // REFACTOR: use crate:: instead of super
use super::base::*;
use super::super::ast;
use crate::names::*;

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
        // `def'
        assert_eq!(*self.current_token(), Token::LowerWord("def"));
        self.consume_token();
        self.skip_ws();

        let sig = self.parse_method_signature()?;
        self.expect_sep()?;

        // Body (optional)
        let body_stmts = self.parse_stmts()?;

        // `end'
        self.skip_wsn();
        match self.current_token() {
            Token::LowerWord("end") => { self.consume_token(); },
            token => return Err(parse_error!(self, "missing `end' of method {:?}; got {:?}", sig.name, token))
        }

        Ok(ast::Definition::InstanceMethodDefinition { sig, body_stmts })
    }

    pub fn parse_method_signature(&mut self) -> Result<ast::MethodSignature, Error> {
        let name;
        let params;
        let ret_typ;

        // Method name
        match self.current_token() {
            Token::LowerWord(s) => { name = MethodName(s.to_string()); self.consume_token(); },
            Token::Symbol(s) => {
                if *s == "+" || *s == "-" || *s == "*" || *s == "/" || *s == "%" {
                    name = MethodName(s.to_string()); self.consume_token();
                }
                else {
                    return Err(parse_error!(self, "method name must start with a-z but got {:?}", s))
                }
            },
            token => return Err(parse_error!(self, "method name must start with a-z but got {:?}", token))
        }
        self.skip_ws();

        // Params (optional)
        match self.current_token() {
            Token::Symbol("(") => { params = self.parse_params()? },
            // Has no params
            _ => { params = vec![]; },
        }
        self.skip_ws();

        // Return type (optional)
        match self.current_token() {
            Token::Symbol("->") => {
                self.consume_token();
                self.skip_ws();
                ret_typ = self.parse_ty()?;
            },
            _ => {
                ret_typ = ast::Typ { name: "Void".to_string() };
                self.skip_ws();
            }
        }

        Ok(ast::MethodSignature { name, params, ret_typ })
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
        let typ = self.parse_ty()?;

        Ok(ast::Param { name, typ })
    }

    fn parse_ty(&mut self) -> Result<ast::Typ, Error> {
        match self.current_token() {
            Token::UpperWord(s) => {
                let typ = ast::Typ { name: s.to_string() };
                self.consume_token();
                Ok(typ)
            },
            token => Err(parse_error!(self, "invalid token as type: {:?}", token))
        }
    }
}
