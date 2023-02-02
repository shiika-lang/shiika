use crate::base::*;
use crate::error::Error;
use crate::Parser;
use shiika_ast::*;
use shiika_core::names::*;

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
            Token::KwModule => Ok(Some(self.parse_module_definition()?)),
            Token::KwEnum => Ok(Some(self.parse_enum_definition()?)),
            Token::KwRequirement => Ok(Some(self.parse_requirement_definition()?)),
            Token::KwDef => Ok(Some(self.parse_method_definition()?)),
            Token::UpperWord(_) => Ok(Some(self.parse_const_definition()?)),
            _ => Ok(None),
        }
    }

    pub fn parse_class_definition(&mut self) -> Result<shiika_ast::Definition, Error> {
        self.debug_log("parse_class_definition");
        self.lv += 1;
        let name;

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

        // Superclass and included modules (optional)
        self.skip_ws()?;
        let supers = if self.current_token_is(Token::Colon) {
            self.consume_token()?;
            self.skip_ws()?;
            self.parse_superclass_and_modules()?
        } else {
            vec![]
        };

        self.expect_sep()?;

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
            supers,
            defs,
        })
    }

    pub fn parse_module_definition(&mut self) -> Result<shiika_ast::Definition, Error> {
        self.debug_log("parse_module_definition");
        self.lv += 1;
        let name;

        // `module'
        assert!(self.consume(Token::KwModule)?);
        self.skip_ws()?;

        // Module name
        match self.current_token() {
            Token::UpperWord(s) => {
                name = module_firstname(s);
                self.consume_token()?;
            }
            token => {
                return Err(parse_error!(
                    self,
                    "module name must start with A-Z but got {:?}",
                    token
                ))
            }
        }

        // Type parameters (optional)
        let typarams = self.parse_opt_typarams()?;

        // Module does not have a superclass
        self.skip_ws()?;
        if self.current_token_is(Token::Colon) {
            return Err(parse_error!(self, "modules does not have superclass"));
        }
        self.expect_sep()?;

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
                    "missing `end' for module {:?}; got {:?}",
                    name,
                    token
                ))
            }
        }

        self.lv -= 1;
        Ok(shiika_ast::Definition::ModuleDefinition {
            name,
            typarams,
            defs,
        })
    }

    pub fn parse_enum_definition(&mut self) -> Result<shiika_ast::Definition, Error> {
        self.debug_log("parse_enum_definition");
        self.lv += 1;
        let name;

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
        let cases = self.parse_enum_cases()?;
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

    /// Parse superclass and included modules of a class.
    fn parse_superclass_and_modules(&mut self) -> Result<Vec<UnresolvedTypeName>, Error> {
        let mut typs = vec![];
        loop {
            typs.push(self.parse_typ()?);
            self.skip_ws()?;
            if self.current_token_is(Token::Comma) {
                self.consume_token()?;
                self.skip_wsn()?;
            } else {
                break;
            }
        }
        Ok(typs)
    }

    /// Parse a method requirement. (must appear only in module definitions)
    fn parse_requirement_definition(&mut self) -> Result<shiika_ast::Definition, Error> {
        self.debug_log("parse_requirement_definition");
        self.lv += 1;
        // `requirement'
        self.set_lexer_state(LexerState::MethodName);
        assert!(self.consume(Token::KwRequirement)?);
        self.skip_ws()?;

        // `foo(bar) -> Baz`
        let (sig, with_self) = self.parse_method_signature()?;
        self.skip_ws()?;
        self.expect_sep()?;
        if with_self {
            return Err(parse_error!(self, "method requirement must not have .self"));
        }

        self.lv -= 1;
        Ok(shiika_ast::Definition::MethodRequirementDefinition { sig })
    }

    /// Parse a method definition.
    pub fn parse_method_definition(&mut self) -> Result<shiika_ast::Definition, Error> {
        self.debug_log("parse_method_definition");
        self.lv += 1;
        // `def'
        self.set_lexer_state(LexerState::MethodName);
        assert!(self.consume(Token::KwDef)?);
        self.skip_ws()?;

        // `foo(bar) -> Baz`
        let (sig, is_class_method) = self.parse_method_signature()?;
        if sig.name.0 != "[]="
            && (sig.name.0.chars().last().unwrap() == '=')
            && sig.params.len() != 1
        {
            return Err(parse_error!(
                self,
                "setter accepts one argument but {:?} were given",
                sig.params.len()
            ));
        }

        self.skip_ws()?;
        self.expect_sep()?;

        // Body (optional)
        let mut body_exprs = self.iparam_exprs(&sig.params);
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
        let is_initializer = sig.name.0 == "initialize";
        if is_class_method {
            if is_initializer {
                let d = shiika_ast::InitializerDefinition { sig, body_exprs };
                Ok(shiika_ast::Definition::ClassInitializerDefinition(d))
            } else {
                Ok(shiika_ast::Definition::ClassMethodDefinition { sig, body_exprs })
            }
        } else {
            if is_initializer {
                let d = shiika_ast::InitializerDefinition { sig, body_exprs };
                Ok(shiika_ast::Definition::InitializerDefinition(d))
            } else {
                Ok(shiika_ast::Definition::InstanceMethodDefinition { sig, body_exprs })
            }
        }
    }

    /// `def initialize(@a: Int)` is equivalent to
    /// `def initialize(a: Int); @a = a`.
    /// Returns expressions like `@a = a`
    fn iparam_exprs(&self, params: &[Param]) -> Vec<shiika_ast::AstExpression> {
        let readonly = true; // TODO: Allow `(var @a: Int)`?
        params
            .iter()
            .filter(|param| param.is_iparam)
            .map(|param| {
                let loc = Location {
                    pos: 0,
                    col: 0,
                    line: 0,
                };
                self.ast.ivar_decl(
                    param.name.clone(),
                    self.ast.bare_name(&param.name, loc.clone(), loc.clone()),
                    readonly,
                    loc.clone(),
                    loc.clone(),
                )
            })
            .collect()
    }

    // Parse `foo(bar) -> Baz`
    pub fn parse_method_signature(
        &mut self,
    ) -> Result<(shiika_ast::AstMethodSignature, bool), Error> {
        let mut name = None;
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
        let params = match self.current_token() {
            Token::LParen => {
                self.consume_token()?;
                self.skip_wsn()?;
                let is_initialize =
                    !is_class_method && name == Some(method_firstname("initialize"));
                self.parse_params(is_initialize, vec![Token::RParen])?
            }
            _ => vec![],
        };
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

    pub(super) fn get_method_name(&mut self) -> Result<&str, Error> {
        let name = match self.current_token() {
            Token::LowerWord(s) => s,
            // Keywords
            Token::KwClass => "class",
            Token::KwEnum => "enum",
            Token::KwCase => "case",
            Token::KwIn => "in",
            Token::KwOut => "out",
            Token::KwEnd => "end",
            Token::KwDef => "def",
            Token::KwVar => "var",
            Token::KwAnd => "and",
            Token::KwOr => "or",
            Token::KwIf => "if",
            Token::KwUnless => "unless",
            Token::KwMatch => "match",
            Token::KwWhen => "when",
            Token::KwWhile => "while",
            Token::KwBreak => "break",
            Token::KwReturn => "return",
            Token::KwThen => "then",
            Token::KwElse => "else",
            Token::KwElsif => "elsif",
            Token::KwFn => "fn",
            Token::KwDo => "do",
            Token::KwSelf => "self",
            Token::KwTrue => "true",
            Token::KwFalse => "false",
            // Symbols
            Token::UPlusMethod => "+@",
            Token::UMinusMethod => "-@",
            Token::GetMethod => "[]",
            Token::SetMethod => "[]=",
            Token::Specialize => "<>",
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
    fn parse_opt_typarams(&mut self) -> Result<Vec<AstTyParam>, Error> {
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
                        None => AstVariance::Invariant,
                        Some(Token::KwOut) => AstVariance::Covariant,
                        Some(Token::KwIn) => AstVariance::Contravariant,
                        _ => panic!("[BUG] unexpected variance token"),
                    };
                    typarams.push(AstTyParam {
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
                    Token::KeyName(_) => params.push(self.parse_param()?),
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
            Token::KeyName(s) => {
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
        // `:'
        if is_iparam {
            self.expect(Token::Colon)?;
        }
        self.skip_ws()?;

        // Type
        let typ = self.parse_typ()?;
        self.skip_ws()?;

        // Default expr (optional)
        let default_expr = if self.consume(Token::Equal)? {
            self.skip_ws()?;
            Some(self.parse_expr()?)
        } else {
            None
        };

        Ok(shiika_ast::Param {
            name,
            typ,
            is_iparam,
            default_expr,
        })
    }

    pub(super) fn parse_typ(&mut self) -> Result<UnresolvedTypeName, Error> {
        match self.current_token() {
            Token::UpperWord(s) => {
                let begin = self.lexer.location();
                let head = s.to_string();
                self.consume_token()?;
                self.set_lexer_gtgt_mode(true); // Prevent `>>` is parsed as RShift
                let name = self._parse_typ(head, begin)?;
                self.set_lexer_gtgt_mode(false); // End special mode
                Ok(name)
            }
            Token::ColonColon => Err(parse_error!(self, "TODO: parse types starting with `::'")),
            token => Err(parse_error!(self, "invalid token as type: {:?}", token)),
        }
    }

    /// Parse a constant name. `s` must be consumed beforehand
    fn _parse_typ(&mut self, s: String, begin: Location) -> Result<UnresolvedTypeName, Error> {
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
                    let inner_begin = self.lexer.location();
                    let name = s.to_string();
                    self.consume_token()?;
                    if lessthan_seen {
                        let inner = self._parse_typ(name, inner_begin)?;
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
        let end = self.lexer.location();
        Ok(self.ast.unresolved_type_name(names, args, begin, end))
    }

    pub fn parse_const_definition(&mut self) -> Result<shiika_ast::Definition, Error> {
        self.debug_log("parse_const_definition");
        self.lv += 1;
        let name = match self.current_token() {
            Token::UpperWord(s) => s.to_string(),
            _ => panic!("must be called on an UpperWord"),
        };
        self.consume_token()?;

        self.skip_wsn()?;
        self.expect(Token::Equal)?;
        self.skip_wsn()?;

        let expr = self.parse_expr()?;

        self.lv -= 1;
        Ok(shiika_ast::Definition::ConstDefinition { name, expr })
    }
}
