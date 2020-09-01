use super::super::ast;
use super::base::*;
use super::Parser; // REFACTOR: use crate:: instead of super
use crate::names::*;

impl<'a> Parser<'a> {
    pub fn parse_definitions(&mut self) -> Result<Vec<ast::Definition>, Error> {
        let mut defs = vec![];
        while let Some(def) = self.parse_definition()? {
            defs.push(def);
            self.skip_wsn();
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
        self.debug_log("parse_class_definition");
        self.lv += 1;
        let name;
        let mut typarams = vec![];
        let defs;

        // `class'
        assert!(self.consume(Token::KwClass));
        self.skip_ws();

        // Class name
        match self.current_token() {
            Token::UpperWord(s) => {
                name = class_firstname(s);
                self.consume_token();
            }
            token => {
                return Err(parse_error!(
                    self,
                    "class name must start with A-Z but got {:?}",
                    token
                ))
            }
        }

        // Type parameters (optional)
        if self.current_token_is(Token::LessThan) {
            self.consume_token();
            self.skip_wsn();
            loop {
                match self.current_token() {
                    Token::GreaterThan => {
                        self.consume_token();
                        break;
                    }
                    Token::UpperWord(s) => {
                        typarams.push(s.to_string());
                        self.consume_token();
                        self.skip_wsn();
                    }
                    Token::Comma => {
                        self.consume_token();
                        self.skip_wsn();
                    }
                    token => {
                        return Err(parse_error!(
                            self,
                            "unexpected token `{:?}' in type parameter definition",
                            token
                        ))
                    }
                }
            }
        }

        // Superclass name (optional)
        let mut super_name = class_fullname("Object");
        self.skip_ws();
        if self.current_token_is(Token::Colon) {
            self.consume_token();
            self.skip_wsn();
            match self.current_token() {
                Token::UpperWord(s) => {
                    super_name = class_fullname(s);
                    self.consume_token();
                }
                token => {
                    return Err(parse_error!(
                        self,
                        "superclass name must start with A-Z but got {:?}",
                        token
                    ))
                }
            }
        }

        self.expect_sep()?;

        // Internal definitions
        defs = self.parse_definitions()?;

        // `end'
        match self.current_token() {
            Token::KwEnd => {
                self.consume_token();
            }
            token => {
                return Err(parse_error!(
                    self,
                    "missing `end' for class {:?}; got {:?}",
                    name,
                    token
                ))
            }
        }

        self.lv -= 1;
        Ok(ast::Definition::ClassDefinition {
            name,
            typarams,
            super_name,
            defs,
        })
    }

    pub fn parse_method_definition(&mut self) -> Result<ast::Definition, Error> {
        self.debug_log("parse_method_definition");
        self.lv += 1;
        // `def'
        self.set_lexer_state(LexerState::MethodName);
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
            Token::KwEnd => {
                self.consume_token();
            }
            token => {
                return Err(parse_error!(
                    self,
                    "missing `end' of method {:?}; got {:?}",
                    sig.name,
                    token
                ))
            }
        }

        self.lv -= 1;
        if is_class_method {
            Ok(ast::Definition::ClassMethodDefinition { sig, body_exprs })
        } else {
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
            if self.current_token_is(Token::Dot) {
                is_class_method = true;
                self.set_lexer_state(LexerState::MethodName);
                self.consume_token();
            } else {
                // Defining a method named `self` :thinking_face:
                name = Some(method_firstname("self"));
            }
        }

        // Method name
        if name == None {
            name = Some(method_firstname(self.get_method_name()?));
            self.consume_token();
        }
        self.skip_ws();

        // Params (optional)
        match self.current_token() {
            Token::LParen => {
                self.consume_token();
                self.skip_wsn();
                params = self.parse_params()?;
            }
            // Has no params
            _ => {
                params = vec![];
            }
        }
        self.skip_ws();

        // Return type (optional)
        match self.current_token() {
            Token::RightArrow => {
                self.consume_token();
                self.skip_ws();
                ret_typ = self.parse_typ()?;
            }
            _ => {
                ret_typ = ast::Typ {
                    name: "Void".to_string(),
                    typ_args: vec![],
                };
                self.skip_ws();
            }
        }

        let sig = ast::AstMethodSignature {
            name: name.unwrap(),
            params,
            ret_typ,
        };
        Ok((sig, is_class_method))
    }

    fn get_method_name(&mut self) -> Result<&str, Error> {
        let name = match self.current_token() {
            Token::LowerWord(s) => s,
            Token::UPlusMethod => "+@",
            Token::UMinusMethod => "-@",
            Token::BinaryPlus => "+",
            Token::BinaryMinus => "-",
            Token::Mul => "*",
            Token::Div => "/",
            Token::Mod => "%",
            Token::And => "&",
            Token::Or => "|",
            Token::Xor => "^",
            Token::LShift => "<<",
            Token::RShift => ">>",
            Token::LessThan => "<",
            Token::LessEq => "<=",
            Token::GreaterThan => ">",
            Token::GreaterEq => ">=",
            Token::EqEq => "==",
            Token::NotEq => "!=",
            token => return Err(parse_error!(self, "invalid method name {:?}", token)),
        };
        Ok(name)
    }

    // Parse parameters
    // The `(` should be consumed beforehand
    pub(super) fn parse_params(&mut self) -> Result<Vec<ast::Param>, Error> {
        let mut params = vec![];
        loop {
            // Param
            match self.current_token() {
                Token::LowerWord(_) => params.push(self.parse_param()?),
                Token::RParen => {
                    self.consume_token();
                    break;
                }
                token => {
                    return Err(parse_error!(
                        self,
                        "invalid token in method arguments: {:?}",
                        token
                    ))
                }
            }
            self.skip_wsn();
            match self.current_token() {
                Token::Comma => {
                    self.consume_token();
                    self.skip_wsn();
                }
                Token::RParen => {
                    self.consume_token();
                    break;
                }
                token => {
                    return Err(parse_error!(
                        self,
                        "invalid token in method arguments: {:?}",
                        token
                    ))
                }
            }
        }
        Ok(params)
    }

    fn parse_param(&mut self) -> Result<ast::Param, Error> {
        let name;

        // Name
        match self.current_token() {
            Token::LowerWord(s) => {
                name = s.to_string();
                self.consume_token();
            }
            token => {
                return Err(parse_error!(
                    self,
                    "invalid token as method param: {:?}",
                    token
                ))
            }
        }
        self.skip_ws();

        // `:'
        self.expect(Token::Colon)?;
        self.skip_ws();

        // Type
        let typ = self.parse_typ()?;

        Ok(ast::Param { name, typ })
    }

    fn parse_typ(&mut self) -> Result<ast::Typ, Error> {
        let mut name = String::new();
        loop {
            match self.current_token() {
                Token::UpperWord(s) => {
                    name += s;
                    self.consume_token();
                }
                Token::ColonColon => {
                    name += "::";
                    self.consume_token();
                }
                Token::LessThan => {
                    self.consume_token();
                    let typ_args = self.parse_typ_args()?;
                    return Ok(ast::Typ { name, typ_args });
                }
                token => {
                    if name.is_empty() {
                        return Err(parse_error!(self, "invalid token as type: {:?}", token));
                    } else {
                        return Ok(ast::Typ {
                            name,
                            typ_args: vec![],
                        });
                    }
                }
            }
        }
    }

    fn parse_typ_args(&mut self) -> Result<Vec<ast::Typ>, Error> {
        let mut typ_args = vec![];
        loop {
            self.skip_wsn();
            typ_args.push(self.parse_typ()?);
            self.skip_wsn();
            match self.current_token() {
                Token::Comma => {
                    self.consume_token();
                }
                Token::GreaterThan => {
                    self.consume_token();
                    return Ok(typ_args);
                }
                token => {
                    return Err(parse_error!(
                        self,
                        "invalid token in type args: {:?}",
                        token
                    ));
                }
            }
        }
    }

    fn parse_const_definition(&mut self) -> Result<ast::Definition, Error> {
        self.debug_log("parse_const_definition");
        self.lv += 1;
        let name;
        match self.current_token() {
            Token::UpperWord(s) => {
                name = const_firstname(s);
            }
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
