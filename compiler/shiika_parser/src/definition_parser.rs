use crate::base::*;
use crate::error::Error;
use crate::Parser;
use shiika_ast;
use shiika_ast::*;
use shiika_core::names::*;
use shiika_core::ty::{TyParam, Variance};

impl<'a> Parser<'a> {
    pub fn parse_definitions(&mut self) -> Result<Vec<shiika_ast::Definition>, Error> {
        let mut defs = vec![];
        while let Some(def) = self.parse_definition()? {
            defs.push(def);
            self.skip_wsn()?;
        }
        Ok(defs)
    }

    fn parse_definition(&mut self) -> Result<Option<shiika_ast::Definition>, Error> {
        match self.current_token() {
            Token::KwClass => Ok(Some(self.parse_class_definition()?)),
            Token::KwEnum => Ok(Some(self.parse_enum_definition()?)),
            Token::KwDef => Ok(Some(self.parse_method_definition()?)),
            Token::UpperWord(_) => Ok(Some(self.parse_const_definition()?)),
            _ => Ok(None),
        }
    }

    pub fn parse_class_definition(&mut self) -> Result<shiika_ast::Definition, Error> {
        self.debug_log("parse_class_definition");
        self.lv += 1;
        let name;
        let defs;

        // `class'
        assert!(self.consume(Token::KwClass)?);
        self.skip_ws()?;

        // Class name
        match self.current_token() {
            Token::UpperWord(s) => {
                name = class_firstname(s);
                self.consume_token()?;
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
        let typarams = self.parse_opt_typarams()?;

        // Superclass name (optional)
        let mut superclass = None;
        self.skip_ws()?;
        if self.current_token_is(Token::Colon) {
            self.consume_token()?;
            self.skip_wsn()?;
            superclass = Some(self.parse_typ()?);
        }

        self.expect_sep()?;

        // Internal definitions
        defs = self.parse_definitions()?;

        // `end'
        match self.current_token() {
            Token::KwEnd => {
                self.consume_token()?;
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
        Ok(shiika_ast::Definition::ClassDefinition {
            name,
            typarams,
            superclass,
            defs,
        })
    }

    pub fn parse_enum_definition(&mut self) -> Result<shiika_ast::Definition, Error> {
        self.debug_log("parse_enum_definition");
        self.lv += 1;
        let name;
        let cases;

        // `enum'
        assert!(self.consume(Token::KwEnum)?);
        self.skip_ws()?;

        // Enum class name
        match self.current_token() {
            Token::UpperWord(s) => {
                name = class_firstname(s);
                self.consume_token()?;
            }
            token => {
                return Err(parse_error!(
                    self,
                    "enum name must start with A-Z but got {:?}",
                    token
                ))
            }
        }

        // Type parameters (optional)
        let typarams = self.parse_opt_typarams()?;
        self.expect_sep()?;

        // Enum cases
        self.skip_wsn()?;
        cases = self.parse_enum_cases()?;
        self.skip_wsn()?;

        // Internal definitions
        let defs = self.parse_definitions()?;

        // `end'
        match self.current_token() {
            Token::KwEnd => {
                self.consume_token()?;
            }
            token => {
                return Err(parse_error!(
                    self,
                    "missing `end' for enum {:?}; got {:?}",
                    name,
                    token
                ))
            }
        }

        self.lv -= 1;
        Ok(shiika_ast::Definition::EnumDefinition {
            name,
            typarams,
            cases,
            defs,
        })
    }

    fn parse_enum_cases(&mut self) -> Result<Vec<shiika_ast::EnumCase>, Error> {
        let mut cases = vec![];
        loop {
            match self.current_token() {
                Token::KwCase => {
                    cases.push(self.parse_enum_case()?);
                    self.skip_wsn()?;
                }
                Token::KwEnd => {
                    break;
                }
                _ => break,
            }
        }
        Ok(cases)
    }

    fn parse_enum_case(&mut self) -> Result<shiika_ast::EnumCase, Error> {
        debug_assert!(self.consume(Token::KwCase)?);
        self.skip_wsn()?;
        let name;
        match self.current_token() {
            Token::UpperWord(s) => {
                name = class_firstname(s);
                self.consume_token()?;
            }
            token => {
                return Err(parse_error!(
                    self,
                    "enum case name must start with A-Z but got {:?}",
                    token
                ))
            }
        }
        let params = match self.current_token() {
            Token::Separator => vec![],
            Token::LParen => {
                self.consume_token()?;
                let is_initialize = false;
                self.parse_params(is_initialize, vec![Token::RParen])?
            }
            token => {
                return Err(parse_error!(
                    self,
                    "unexpected token after enum case name: {:?}",
                    token
                ))
            }
        };
        Ok(shiika_ast::EnumCase { name, params })
    }

    pub fn parse_method_definition(&mut self) -> Result<shiika_ast::Definition, Error> {
        self.debug_log("parse_method_definition");
        self.lv += 1;
        // `def'
        self.set_lexer_state(LexerState::MethodName);
        assert!(self.consume(Token::KwDef)?);
        self.skip_ws()?;

        // `foo(bar) -> Baz`
        let (sig, is_class_method) = self.parse_method_signature()?;
        self.skip_ws()?;
        self.expect_sep()?;

        // Body (optional)
        let mut body_exprs = iparam_exprs(&sig.params);
        body_exprs.append(&mut self.parse_exprs(vec![Token::KwEnd])?);

        // `end'
        self.skip_wsn()?;
        match self.current_token() {
            Token::KwEnd => {
                self.consume_token()?;
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
            Ok(shiika_ast::Definition::ClassMethodDefinition { sig, body_exprs })
        } else {
            Ok(shiika_ast::Definition::InstanceMethodDefinition { sig, body_exprs })
        }
    }

    // Parse `foo(bar) -> Baz`
    pub fn parse_method_signature(
        &mut self,
    ) -> Result<(shiika_ast::AstMethodSignature, bool), Error> {
        let mut name = None;
        let params;
        let ret_typ;
        let mut is_class_method = false;

        // `self.` (Optional)
        if self.consume(Token::KwSelf)? {
            if self.current_token_is(Token::Dot) {
                is_class_method = true;
                self.set_lexer_state(LexerState::MethodName);
                self.consume_token()?;
            } else {
                // Defining a method named `self` :thinking_face:
                name = Some(method_firstname("self"));
            }
        }

        // Method name
        if name == None {
            name = Some(method_firstname(self.get_method_name()?));
            self.consume_token()?;
        }

        // Method-wise type parameters (Optional)
        let typarams = self.parse_opt_typarams()?;

        // Params (optional)
        match self.current_token() {
            Token::LParen => {
                self.consume_token()?;
                self.skip_wsn()?;
                let is_initialize =
                    !is_class_method && name == Some(method_firstname("initialize"));
                params = self.parse_params(is_initialize, vec![Token::RParen])?;
            }
            // Has no params
            _ => {
                params = vec![];
            }
        }
        self.skip_ws()?;

        // Return type (optional)
        match self.current_token() {
            Token::RightArrow => {
                self.consume_token()?;
                self.skip_ws()?;
                ret_typ = Some(self.parse_typ()?);
            }
            _ => {
                ret_typ = None;
                self.skip_ws()?;
            }
        }

        let sig = shiika_ast::AstMethodSignature {
            name: name.unwrap(),
            typarams,
            params,
            ret_typ,
        };
        Ok((sig, is_class_method))
    }

    fn get_method_name(&mut self) -> Result<&str, Error> {
        let name = match self.current_token() {
            Token::LowerWord(s) => s,
            Token::KwClass => "class",
            Token::UPlusMethod => "+@",
            Token::UMinusMethod => "-@",
            Token::GetMethod => "[]",
            Token::SetMethod => "[]=",
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

    // Parse type parameters of a class or a method
    // - `class Foo<A, B, C>`
    // - `def foo<A, B, C>( ... )`
    fn parse_opt_typarams(&mut self) -> Result<Vec<TyParam>, Error> {
        if !self.current_token_is(Token::LessThan) {
            return Ok(Default::default());
        }
        let mut typarams = vec![];
        let mut variance = None;
        debug_assert!(self.consume(Token::LessThan)?);
        self.skip_wsn()?;
        loop {
            let token = self.current_token();
            match token {
                Token::GreaterThan => {
                    self.consume_token()?;
                    break;
                }
                Token::UpperWord(s) => {
                    let v = match variance {
                        None => Variance::Invariant,
                        Some(Token::KwOut) => Variance::Covariant,
                        Some(Token::KwIn) => Variance::Contravariant,
                        _ => panic!("[BUG] unexpected variance token"),
                    };
                    typarams.push(TyParam {
                        name: s.to_string(),
                        variance: v,
                    });
                    variance = None;
                    self.consume_token()?;
                    self.skip_wsn()?;
                }
                Token::Comma => {
                    self.consume_token()?;
                    self.skip_wsn()?;
                }
                Token::KwIn | Token::KwOut => {
                    if let Some(t) = variance {
                        return Err(parse_error!(
                            self,
                            "unexpected token `{:?}' after `{:?}'",
                            token,
                            t
                        ));
                    }
                    variance = Some(token.clone());
                    self.consume_token()?;
                    self.expect(Token::Space)?;
                }
                _ => {
                    return Err(parse_error!(
                        self,
                        "unexpected token `{:?}' in type parameter definition",
                        token
                    ))
                }
            }
        }
        Ok(typarams)
    }

    // Parse parameters
    // - The `(` should be consumed beforehand
    pub(super) fn parse_params(
        &mut self,
        is_initialize: bool,
        stop_toks: Vec<Token>,
    ) -> Result<Vec<shiika_ast::Param>, Error> {
        let mut params = vec![];
        loop {
            // Param
            if !stop_toks.contains(self.current_token()) {
                match self.current_token() {
                    Token::IVar(_) => {
                        if is_initialize {
                            params.push(self.parse_param()?);
                        } else {
                            return Err(parse_error!(self, "@ is only used in `initialize'"));
                        }
                    }
                    Token::LowerWord(_) => params.push(self.parse_param()?),
                    token => {
                        return Err(parse_error!(
                            self,
                            "invalid token in method arguments: {:?}",
                            token
                        ))
                    }
                }
                self.skip_wsn()?;
            }
            // Next param or exit
            if stop_toks.contains(self.current_token()) {
                self.consume_token()?;
                break;
            }
            match self.current_token() {
                Token::Comma => {
                    self.consume_token()?;
                    self.skip_wsn()?;
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

    fn parse_param(&mut self) -> Result<shiika_ast::Param, Error> {
        let name;
        let is_iparam;

        // Name
        match self.current_token() {
            Token::LowerWord(s) => {
                name = s.to_string();
                self.consume_token()?;
                is_iparam = false;
            }
            Token::IVar(s) => {
                name = s.to_string();
                self.consume_token()?;
                is_iparam = true;
            }
            token => {
                return Err(parse_error!(
                    self,
                    "invalid token as method param: {:?}",
                    token
                ))
            }
        }
        self.skip_ws()?;

        // `:'
        self.expect(Token::Colon)?;
        self.skip_ws()?;

        // Type
        let typ = self.parse_typ()?;

        Ok(shiika_ast::Param {
            name,
            typ,
            is_iparam,
        })
    }

    fn parse_typ(&mut self) -> Result<ConstName, Error> {
        match self.current_token() {
            Token::UpperWord(s) => {
                let head = s.to_string();
                self.consume_token()?;
                self.set_lexer_gtgt_mode(true); // Prevent `>>` is parsed as RShift
                let name = self._parse_typ(head)?;
                self.set_lexer_gtgt_mode(false); // End special mode
                Ok(name)
            }
            Token::ColonColon => Err(parse_error!(self, "TODO: parse types starting with `::'")),
            token => Err(parse_error!(self, "invalid token as type: {:?}", token)),
        }
    }

    /// Parse a constant name. `s` must be consumed beforehand
    fn _parse_typ(&mut self, s: String) -> Result<ConstName, Error> {
        self.lv += 1;
        self.debug_log("_parse_typ");
        let mut names = vec![s];
        let mut lessthan_seen = false;
        let mut args = vec![];
        loop {
            let tok = self.current_token();
            match tok {
                Token::ColonColon => {
                    // `A::B`
                    if lessthan_seen {
                        return Err(parse_error!(self, "unexpected `::'"));
                    }
                    self.consume_token()?;
                }
                Token::LessThan => {
                    // `A<B>`
                    lessthan_seen = true;
                    self.consume_token()?;
                    self.skip_wsn()?;
                }
                Token::GreaterThan => {
                    // `A<B>`
                    if lessthan_seen {
                        self.consume_token()?;
                    }
                    break;
                }
                Token::Comma => {
                    // `A<B, C>`
                    if lessthan_seen {
                        self.consume_token()?;
                        self.skip_wsn()?;
                    } else {
                        break;
                    }
                }
                Token::UpperWord(s) => {
                    let name = s.to_string();
                    self.consume_token()?;
                    if lessthan_seen {
                        let inner = self._parse_typ(name)?;
                        args.push(inner);
                        self.skip_wsn()?;
                    } else {
                        names.push(name);
                    }
                }
                token => {
                    if lessthan_seen {
                        return Err(parse_error!(self, "unexpected token: {:?}", token));
                    } else {
                        break;
                    }
                }
            }
        }
        self.lv -= 1;
        Ok(ConstName { names, args })
    }

    pub fn parse_const_definition(&mut self) -> Result<shiika_ast::Definition, Error> {
        self.debug_log("parse_const_definition");
        self.lv += 1;
        let name;
        match self.current_token() {
            Token::UpperWord(s) => name = s.to_string(),
            _ => panic!("must be called on an UpperWord"),
        }
        self.consume_token()?;

        self.skip_wsn()?;
        self.expect(Token::Equal)?;
        self.skip_wsn()?;

        let expr = self.parse_expr()?;

        self.lv -= 1;
        Ok(shiika_ast::Definition::ConstDefinition { name, expr })
    }
}

/// `def initialize(@a: Int)` is equivalent to
/// `def initialize(a: Int); @a = a`.
/// Returns expressions like `@a = a`
fn iparam_exprs(params: &[Param]) -> Vec<shiika_ast::AstExpression> {
    params
        .iter()
        .filter(|param| param.is_iparam)
        .map(|param| {
            shiika_ast::ivar_assign(param.name.clone(), shiika_ast::bare_name(&param.name))
        })
        .collect()
}
