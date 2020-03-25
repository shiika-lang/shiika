use super::Parser; // REFACTOR: use crate:: instead of super
use super::base::*;
use super::super::ast;
use crate::names::*;

impl<'a> Parser<'a> {
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
            Token::KwClass => Ok(Some(self.parse_class_definition()?)),
            Token::KwDef => Ok(Some(self.parse_method_definition()?)),
            Token::UpperWord(_) => Ok(Some(self.parse_const_definition()?)),
            _ => Ok(None),
        }
    }

    pub fn parse_class_definition(&mut self) -> Result<ast::Definition, Error> {
        self.debug_log("parse_class_definition"); self.lv += 1;
        let name;
        let defs;

        // `class'
        assert!(self.consume(Token::KwClass));
        self.skip_ws();

        // Class name
        match self.current_token() {
            Token::UpperWord(s) => {
                name = ClassFirstname(s.to_string());
                self.consume_token();
            },
            token => return Err(parse_error!(self, "class name must start with A-Z but got {:?}", token))
        }
        self.expect_sep()?;

        // Internal definitions
        defs = self.parse_definitions()?;

        // `end'
        match self.current_token() {
            Token::KwEnd => { self.consume_token(); },
            token => return Err(parse_error!(self, "missing `end' for class {:?}; got {:?}", name, token))
        }
        
        self.lv -= 1;
        Ok(ast::Definition::ClassDefinition { name, defs })
    }

    pub fn parse_method_definition(&mut self) -> Result<ast::Definition, Error> {
        self.debug_log("parse_method_definition"); self.lv += 1;
        // `def'
        assert!(self.consume(Token::KwDef));
        self.skip_ws();

        // `foo(bar) -> Baz`
        let (sig, is_class_method) = self.parse_method_signature()?;
        self.expect_sep()?;

        // Body (optional)
        let body_exprs = self.parse_exprs(vec![Token::KwEnd])?;

        // `end'
        self.skip_wsn();
        match self.current_token() {
            Token::KwEnd => { self.consume_token(); },
            token => return Err(parse_error!(self, "missing `end' of method {:?}; got {:?}", sig.name, token))
        }

        self.lv -= 1;
        if is_class_method {
            Ok(ast::Definition::ClassMethodDefinition { sig, body_exprs })
        }
        else {
            Ok(ast::Definition::InstanceMethodDefinition { sig, body_exprs })
        }
    }

    pub fn parse_method_signature(&mut self) -> Result<(ast::AstMethodSignature, bool), Error> {
        let mut name = None;
        let params;
        let ret_typ;
        let mut is_class_method = false;

        // `self.` (Optional)
        if self.consume(Token::KwSelf) {
            if self.consume(Token::Dot) {
                is_class_method = true;
            }
            else {
                name = Some(MethodFirstname("self".to_string()));
            }
        }

        // Method name
        if name == None {
            name = Some(MethodFirstname(self.get_method_name()?.to_string()));
            self.consume_token();
        }
        self.skip_ws();

        // Params (optional)
        match self.current_token() {
            Token::LParen => { params = self.parse_params()? },
            // Has no params
            _ => { params = vec![]; },
        }
        self.skip_ws();

        // Return type (optional)
        match self.current_token() {
            Token::RightArrow => {
                self.consume_token();
                self.skip_ws();
                ret_typ = self.parse_ty()?;
            },
            _ => {
                ret_typ = ast::Typ { name: "Void".to_string() };
                self.skip_ws();
            }
        }

        let sig = ast::AstMethodSignature { name: name.unwrap(), params, ret_typ };
        Ok((sig, is_class_method))
    }

    fn get_method_name(&self) -> Result<&str, Error> {
        let name = match self.current_token() {
            Token::LowerWord(s) => { s },
            // TODO: `+@`, `-@`
            Token::UnaryPlus => { "+" },
            Token::UnaryMinus => { "-" },
            Token::Mul => { "*" },
            Token::Div => { "/" },
            Token::Mod => { "%" },
            Token::And => { "&" },
            Token::Or => { "|" },
            Token::Xor => { "^" },
            Token::LShift => { "<<" },
            Token::RShift => { ">>" },
            Token::LessThan => { "<" },
            Token::LessEq => { "<=" },
            Token::GraterThan => { ">" },
            Token::GraterEq => { ">=" },
            Token::EqEq => { "==" },
            Token::NotEq => { "!=" },
            token => {
                return Err(parse_error!(self, "invalid method name {:?}", token))
            }
        };
        Ok(name)
    }

    fn parse_params(&mut self) -> Result<Vec<ast::Param>, Error> {
        let mut params = vec!();

        assert!(self.consume(Token::LParen));

        loop {
            // Param
            match self.current_token() {
                Token::LowerWord(_) => { params.push(self.parse_param()?) },
                Token::RParen       => { self.consume_token(); break },
                token => return Err(parse_error!(self, "invalid token in method arguments: {:?}", token))
            }
            self.skip_wsn();
            match self.current_token() {
                Token::Comma => { self.consume_token(); self.skip_wsn(); },
                Token::RParen => { self.consume_token(); break }
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
        self.expect(Token::Colon)?;
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

    fn parse_const_definition(&mut self) -> Result<ast::Definition, Error> {
        self.debug_log("parse_const_definition"); self.lv += 1;
        let name;
        match self.current_token() {
            Token::UpperWord(s) => {
                name = ConstFirstname(s.to_string());
            },
            _ => panic!("must be called on an UpperWord"),
        }
        self.consume_token();

        self.skip_wsn();
        self.expect(Token::Equal)?;
        self.skip_wsn();

        let expr = self.parse_expr()?;

        self.lv -= 1;
        Ok(ast::Definition::ConstDefinition { name, expr })
    }
}
