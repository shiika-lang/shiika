use crate::parser::base::*;
use std::collections::HashMap;

impl<'a> Parser<'a> {
    /// Parse successive expressions
    pub fn parse_exprs(&mut self, stop_toks: Vec<Token>) -> Result<Vec<AstExpression>, Error> {
        let mut ret = Vec::new();
        let mut expr_seen = false;
        loop {
            if stop_toks.contains(self.current_token()) {
                return Ok(ret);
            } else if self.current_token_is(Token::Space) {
                self.consume_token();
            } else if self.current_token_is(Token::Separator) {
                self.consume_token();
                expr_seen = false;
            } else {
                if expr_seen {
                    self.expect_sep()?; // Missing separator between exprs
                }
                ret.push(self.parse_expr()?);
                expr_seen = true;
            }
        }
    }

    pub fn parse_expr(&mut self) -> Result<AstExpression, Error> {
        self.parse_var_decl()
    }

    pub fn parse_var_decl(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_var_decl");
        let expr;
        if self.current_token_is(Token::KwVar) {
            self.consume_token();
            self.skip_ws();
            match self.current_token() {
                Token::LowerWord(s) => {
                    let name = s.to_string();
                    self.consume_token();
                    self.skip_ws();
                    self.expect(Token::Equal)?;
                    self.skip_wsn();
                    let rhs = self.parse_operator_expr()?;
                    expr = ast::lvar_decl(name, rhs);
                }
                Token::IVar(s) => {
                    let name = s.to_string();
                    self.consume_token();
                    self.skip_ws();
                    self.expect(Token::Equal)?;
                    self.skip_wsn();
                    let rhs = self.parse_operator_expr()?;
                    expr = ast::ivar_decl(name, rhs);
                }
                token => return Err(parse_error!(self, "invalid var name: {:?}", token)),
            }
        } else {
            expr = self.parse_and_or_expr()?;
        }
        self.lv -= 1;
        Ok(expr)
    }

    pub fn parse_and_or_expr(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_and_or_expr");
        let mut expr = self.parse_not_expr()?;
        self.skip_ws();
        loop {
            match self.current_token() {
                Token::KwAnd => {
                    self.consume_token();
                    self.skip_wsn();
                    expr = ast::logical_and(expr, self.parse_not_expr()?);
                }
                Token::KwOr => {
                    self.consume_token();
                    self.skip_wsn();
                    expr = ast::logical_or(expr, self.parse_not_expr()?);
                }
                _ => break,
            }
            self.skip_ws();
        }
        self.lv -= 1;
        Ok(expr)
    }

    fn parse_not_expr(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_not_expr");
        let expr = match self.current_token() {
            Token::KwNot => {
                self.consume_token();
                self.skip_ws();
                let inner = self.parse_not_expr()?;
                ast::logical_not(inner)
            }
            Token::Bang => {
                self.consume_token();
                self.skip_ws();
                let inner = self.parse_call_wo_paren()?;
                ast::logical_not(inner)
            }
            _ => self.parse_call_wo_paren()?,
        };
        self.lv -= 1;
        Ok(expr)
    }

    //        methodInvocationWithoutParentheses:
    //                MethodIdentifier bar (do ... end)?
    //                primaryExpression . foo bar (do ... end)?
    //        operatorExpression
    fn parse_call_wo_paren(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_call_wo_paren");

        // If `LowerWord + Space`, see if the rest is an argument list
        let token = self.current_token();
        if let Token::LowerWord(s) = token.clone() {
            let next_token = self.peek_next_token();
            if next_token == Token::Space {
                let cur = self.current_position();
                self.consume_token();
                self.set_lexer_state(LexerState::ExprArg);
                assert!(self.consume(Token::Space));
                let mut args = self.parse_operator_exprs()?;
                self.debug_log(&format!("tried/args: {:?}", args));
                if !args.is_empty() {
                    self.skip_ws();
                    if let Some(lambda) = self.parse_opt_do_block()? {
                        args.push(lambda);
                    }
                    self.lv -= 1;
                    return Ok(ast::method_call(None, &s, args, false, false));
                }
                self.rewind_to(cur)
            }
        }

        // Otherwise, read an expression
        let expr = self.parse_operator_expr()?;

        // See if it is a method invocation (eg. `x.foo 1, 2`)
        if expr.may_have_paren_wo_args() {
            let mut args = self.parse_operator_exprs()?;
            if !args.is_empty() {
                self.skip_ws();
                if let Some(lambda) = self.parse_opt_do_block()? {
                    args.push(lambda);
                }
                self.lv -= 1;
                return Ok(ast::set_method_call_args(expr, args));
            } else if self.next_nonspace_token() == Token::KwDo {
                self.skip_ws();
                let lambda = self.parse_do_block()?;
                return Ok(ast::set_method_call_args(expr, vec![lambda]));
            }
        }

        self.lv -= 1;
        Ok(expr)
    }

    fn parse_args(&mut self) -> Result<Vec<AstExpression>, Error> {
        self.lv += 1;
        self.debug_log("parse_args");
        let expr = self.parse_operator_exprs()?;
        self.lv -= 1;
        Ok(expr)
    }

    /// Parse successive operator_exprs delimited by `,`
    ///
    /// May return empty Vec if there are no values
    fn parse_operator_exprs(&mut self) -> Result<Vec<AstExpression>, Error> {
        self.lv += 1;
        self.debug_log("parse_operator_exprs");
        let mut v = vec![];
        if self.next_nonspace_token().value_starts() {
            v.push(self.parse_operator_expr()?);
            loop {
                self.skip_ws();
                if !self.current_token_is(Token::Comma) {
                    break;
                }
                self.consume_token();
                self.skip_wsn();
                v.push(self.parse_operator_expr()?);
            }
        }
        self.lv -= 1;
        Ok(v)
    }

    // operatorExpression:
    //   assignmentExpression |
    //   conditionalOperatorExpression
    fn parse_operator_expr(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_operator_expr");
        let expr = self.parse_conditional_expr()?;
        if expr.is_lhs() && self.next_nonspace_token().is_assignment_token() {
            self.parse_assignment_expr(expr)
        } else {
            self.lv -= 1;
            Ok(expr)
        }
    }

    // assignmentExpression:
    //       singleAssignmentExpression |
    //       abbreviatedAssignmentExpression |
    //       assignmentWithRescueModifier
    fn parse_assignment_expr(&mut self, lhs: AstExpression) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_assignment_expr");

        self.skip_ws();
        let op = self.next_nonspace_token();
        self.consume_token();
        self.skip_wsn();
        let rhs = self.parse_operator_expr()?;

        self.lv -= 1;

        Ok(match op {
            Token::Equal => ast::assignment(lhs, rhs),
            Token::PlusEq => ast::assignment(lhs.clone(), ast::bin_op_expr(lhs, "+", rhs)),
            Token::MinusEq => ast::assignment(lhs.clone(), ast::bin_op_expr(lhs, "-", rhs)),
            Token::MulEq => ast::assignment(lhs.clone(), ast::bin_op_expr(lhs, "*", rhs)),
            Token::DivEq => ast::assignment(lhs.clone(), ast::bin_op_expr(lhs, "/", rhs)),
            Token::ModEq => ast::assignment(lhs.clone(), ast::bin_op_expr(lhs, "%", rhs)),
            Token::LShiftEq => ast::assignment(lhs.clone(), ast::bin_op_expr(lhs, "<<", rhs)),
            Token::RShiftEq => ast::assignment(lhs.clone(), ast::bin_op_expr(lhs, ">>", rhs)),
            Token::AndEq => ast::assignment(lhs.clone(), ast::bin_op_expr(lhs, "&", rhs)),
            Token::OrEq => ast::assignment(lhs.clone(), ast::bin_op_expr(lhs, "|", rhs)),
            Token::XorEq => ast::assignment(lhs.clone(), ast::bin_op_expr(lhs, "^", rhs)),
            Token::AndAndEq => ast::assignment(lhs.clone(), ast::logical_and(lhs, rhs)),
            Token::OrOrEq => ast::assignment(lhs.clone(), ast::logical_or(lhs, rhs)),
            _unexpected => unimplemented!(),
        })
    }

    /// `a ? b : c`
    fn parse_conditional_expr(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_conditional_expr");
        let expr = self.parse_range_expr()?;
        if self.next_nonspace_token() == Token::Question {
            self.skip_ws();
            assert!(self.consume(Token::Question));
            self.skip_wsn();
            let then_expr = self.parse_operator_expr()?;
            self.skip_ws();
            self.expect(Token::Colon)?;
            self.skip_wsn();
            let else_expr = self.parse_operator_expr()?;
            self.lv -= 1;
            Ok(ast::if_expr(expr, vec![then_expr], Some(vec![else_expr])))
        } else {
            self.lv -= 1;
            Ok(expr)
        }
    }

    /// `a..b`, `a...b`
    fn parse_range_expr(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_range_expr");
        let expr = self.parse_operator_or()?;
        //        self.skip_ws();
        //        match self.current_token() {
        //            Token::symbol(s @ "..") | Token::symbol(s @ "...") => {
        //                let inclusive = (s == "..");
        //                self.skip_wsn();
        //                let end_expr = self.parse_operator_or()?;
        //                Ok(ast::range_expr(Some(expr), Some(end_expr), inclusive))
        //            },
        //            _ => Ok(expr)
        //        }
        self.lv -= 1;
        Ok(expr)
    }

    /// `||`
    fn parse_operator_or(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_operator_or");
        let mut expr = self.parse_operator_and()?;
        let mut token = &self.next_nonspace_token();
        loop {
            if *token == Token::OrOr {
                self.skip_ws();
                assert!(self.consume(Token::OrOr));
                self.skip_wsn();
                expr = ast::logical_or(expr, self.parse_operator_and()?);
                self.skip_ws();
                token = self.current_token();
            } else {
                break;
            }
        }
        self.lv -= 1;
        Ok(expr)
    }

    /// `&&`
    fn parse_operator_and(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_operator_and");
        let mut expr = self.parse_equality_expr()?;
        let mut token = &self.next_nonspace_token();
        loop {
            if *token == Token::AndAnd {
                self.skip_ws();
                assert!(self.consume(Token::AndAnd));
                self.skip_wsn();
                expr = ast::logical_and(expr, self.parse_equality_expr()?);
                self.skip_ws();
                token = self.current_token();
            } else {
                break;
            }
        }
        self.lv -= 1;
        Ok(expr)
    }

    /// `==`, etc.
    fn parse_equality_expr(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_equality_expr");
        let left = self.parse_relational_expr()?;
        let op = match self.next_nonspace_token() {
            // TODO: <=> === =~ !~
            Token::EqEq => "==",
            Token::NotEq => "!=",
            _ => {
                self.lv -= 1;
                return Ok(left);
            }
        };

        self.skip_ws();
        self.consume_token();
        self.skip_wsn();
        let right = self.parse_relational_expr()?;
        let call_eq = ast::method_call(Some(left), "==", vec![right], false, false);
        let expr = if op == "!=" {
            ast::logical_not(call_eq)
        } else {
            call_eq
        };
        self.lv -= 1;
        Ok(expr)
    }

    fn parse_relational_expr(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_relational_expr");
        let mut expr = self.parse_bitwise_or()?; // additive (> >= < <=) additive
        let mut nesting = false;
        loop {
            let op = match self.next_nonspace_token() {
                Token::LessThan => "<",
                Token::GreaterThan => ">",
                Token::LessEq => "<=",
                Token::GreaterEq => ">=",
                _ => break,
            };
            self.skip_ws();
            self.consume_token();
            self.skip_wsn();
            let right = self.parse_bitwise_or()?;

            if nesting {
                if let AstExpressionBody::MethodCall { arg_exprs, .. } = &expr.body {
                    let mid = arg_exprs[0].clone();
                    let compare = ast::method_call(Some(mid), op, vec![right], false, false);
                    expr = ast::logical_and(expr, compare);
                }
            } else {
                expr = ast::method_call(Some(expr), op, vec![right], false, false);
                nesting = true;
            }
        }
        self.lv -= 1;
        Ok(expr)
    }

    fn parse_bitwise_or(&mut self) -> Result<AstExpression, Error> {
        let mut symbols = HashMap::new();
        symbols.insert(Token::Or, "|");
        symbols.insert(Token::Xor, "^");
        self.parse_binary_operator("parse_bitwise_or", Parser::parse_bitwise_and, symbols)
    }

    fn parse_bitwise_and(&mut self) -> Result<AstExpression, Error> {
        let mut symbols = HashMap::new();
        symbols.insert(Token::And, "&");
        self.parse_binary_operator("parse_bitwise_and", Parser::parse_bitwise_shift, symbols)
    }

    fn parse_bitwise_shift(&mut self) -> Result<AstExpression, Error> {
        let mut symbols = HashMap::new();
        symbols.insert(Token::LShift, "<<");
        symbols.insert(Token::RShift, ">>");
        self.parse_binary_operator("parse_bitwise_shift", Parser::parse_additive_expr, symbols)
    }

    fn parse_additive_expr(&mut self) -> Result<AstExpression, Error> {
        let mut symbols = HashMap::new();
        symbols.insert(Token::BinaryPlus, "+");
        symbols.insert(Token::BinaryMinus, "-");
        self.parse_binary_operator(
            "parse_additive_expr",
            Parser::parse_multiplicative_expr,
            symbols,
        )
    }

    fn parse_multiplicative_expr(&mut self) -> Result<AstExpression, Error> {
        let mut symbols = HashMap::new();
        symbols.insert(Token::Mul, "*");
        symbols.insert(Token::Div, "/");
        symbols.insert(Token::Mod, "%");
        self.parse_binary_operator(
            "parse_multiplicative_expr",
            Parser::parse_unary_minus_expr,
            symbols,
        )
    }

    fn parse_unary_minus_expr(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_unary_minus_expr");
        //TODO:
        //  parse_unary_minus_expr
        //  parse_power_expr
        //  parse_unary_expr
        //  parse_secondary_expr
        let expr = if self.consume(Token::UnaryMinus) {
            let target = self.parse_secondary_expr()?;
            ast::unary_expr(target, "-@")
        } else {
            self.parse_secondary_expr()?
        };
        self.lv -= 1;
        Ok(expr)
    }

    /// Secondary expression
    ///
    /// Mostly primary but cannot be a method receiver
    /// eg.
    ///    NG: if foo then bar else baz end.quux()
    ///    OK: (if foo then bar else baz end).quux()
    fn parse_secondary_expr(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_secondary_expr");
        let expr = match self.current_token() {
            Token::KwBreak => self.parse_break_expr(),
            Token::KwIf => self.parse_if_expr(),
            Token::KwUnless => self.parse_unless_expr(),
            Token::KwWhile => self.parse_while_expr(),
            _ => self.parse_primary_expr(),
        }?;
        self.lv -= 1;
        Ok(expr)
    }

    fn parse_break_expr(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_break_expr");
        assert!(self.consume(Token::KwBreak));
        Ok(ast::break_expr())
    }

    fn parse_if_expr(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_if_expr");
        assert!(self.consume(Token::KwIf));
        self.skip_ws();
        let cond_expr = self.parse_expr()?;
        self.skip_ws();
        if self.consume(Token::KwThen) {
            self.skip_wsn();
        } else {
            self.expect(Token::Separator)?;
        }
        let then_exprs = self.parse_exprs(vec![Token::KwEnd, Token::KwElse])?;
        self.skip_wsn();
        if self.consume(Token::KwElse) {
            self.skip_wsn();
            let else_exprs = self.parse_exprs(vec![Token::KwEnd])?;
            self.skip_wsn();
            self.expect(Token::KwEnd)?;
            self.lv -= 1;
            Ok(ast::if_expr(cond_expr, then_exprs, Some(else_exprs)))
        } else {
            self.expect(Token::KwEnd)?;
            self.lv -= 1;
            Ok(ast::if_expr(cond_expr, then_exprs, None))
        }
    }

    fn parse_unless_expr(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_unlessif_expr");
        assert!(self.consume(Token::KwUnless));
        self.skip_ws();
        let cond_expr = self.parse_expr()?;
        self.skip_ws();
        if self.consume(Token::KwThen) {
            self.skip_wsn();
        } else {
            self.expect(Token::Separator)?;
        }
        let then_exprs = self.parse_exprs(vec![Token::KwEnd, Token::KwElse])?;
        self.skip_wsn();
        if self.consume(Token::KwElse) {
            Err(parse_error!(self, "unless cannot have a else clause"))
        } else {
            self.expect(Token::KwEnd)?;
            self.lv -= 1;
            Ok(ast::if_expr(ast::logical_not(cond_expr), then_exprs, None))
        }
    }

    fn parse_while_expr(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_while_expr");
        assert!(self.consume(Token::KwWhile));
        self.skip_ws();
        let cond_expr = self.parse_expr()?;
        self.skip_ws();
        self.expect(Token::Separator)?;
        let body_exprs = self.parse_exprs(vec![Token::KwEnd])?;
        self.skip_wsn();
        self.expect(Token::KwEnd)?;
        self.lv -= 1;
        Ok(ast::while_expr(cond_expr, body_exprs))
    }

    // prim . methodName argumentWithParentheses? block?
    // prim [ indexingArgumentList? ] not(EQUAL)
    fn parse_primary_expr(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_primary_expr");
        let mut expr = self.parse_atomic()?;
        loop {
            if self.next_nonspace_token() == Token::Dot {
                // TODO: Newline should also be allowed here (but Semicolon is not)
                self.skip_ws();
                expr = self.parse_method_chain(expr)?;
            } else {
                break;
            }
        }
        self.lv -= 1;
        Ok(expr)
    }

    /// Parse `.foo(args)` plus a block, if any
    fn parse_method_chain(&mut self, expr: AstExpression) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_method_chain");
        // .
        assert!(self.consume(Token::Dot));
        self.skip_wsn();

        // Method name
        let method_name = match self.current_token() {
            Token::LowerWord(s) => s.clone(),
            token => return Err(parse_error!(self, "invalid method name: {:?}", token)),
        };
        self.consume_token();

        // Args
        let (mut args, may_have_paren_wo_args) = match self.current_token() {
            // .foo(args)
            Token::LParen => (self.parse_paren_and_args()?, false),
            // .foo
            _ => (vec![], true),
        };

        // Block
        if let Some(lambda) = self.parse_opt_block()? {
            args.push(lambda)
        }

        self.lv -= 1;
        Ok(ast::method_call(
            Some(expr),
            &method_name,
            args,
            true,
            may_have_paren_wo_args,
        ))
    }

    fn parse_paren_and_args(&mut self) -> Result<Vec<AstExpression>, Error> {
        self.lv += 1;
        self.debug_log("parse_paren_and_args");
        assert!(self.consume(Token::LParen));
        self.skip_wsn();
        let args;
        if self.consume(Token::RParen) {
            args = vec![]
        } else {
            args = self.parse_args()?;
            self.skip_wsn();
            self.expect(Token::RParen)?;
        }
        self.lv -= 1;
        Ok(args)
    }

    fn parse_atomic(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_atomic");
        let token = self.current_token();
        let expr = match token {
            Token::LowerWord(s) => {
                let name = s.to_string();
                self.consume_token();
                self.parse_primary_method_call(&name)
            }
            Token::UpperWord(s) => {
                let name = s.to_string();
                self.parse_const_ref(name)
            }
            Token::KwFn => self.parse_lambda(),
            Token::KwSelf | Token::KwTrue | Token::KwFalse => {
                let t = token.clone();
                self.consume_token();
                Ok(ast::pseudo_variable(t))
            }
            Token::IVar(s) => {
                let name = s.to_string();
                self.consume_token();
                Ok(ast::ivar_ref(name))
            }
            Token::LSqBracket => self.parse_array_literal(),
            Token::Number(_) => self.parse_decimal_literal(),
            Token::Str(_) => self.parse_string(),
            Token::LParen => self.parse_parenthesized_expr(),
            token => Err(parse_error!(self, "unexpected token: {:?}", token)),
        }?;
        self.lv -= 1;
        Ok(expr)
    }

    // Method call with explicit parenthesis (eg. `foo(bar)`) optionally followed by a block
    fn parse_primary_method_call(&mut self, bare_name_str: &str) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_primary_method_call");
        let expr = match self.current_token() {
            Token::LParen => {
                let mut args = self.parse_paren_and_args()?;
                if let Some(lambda) = self.parse_opt_block()? {
                    args.push(lambda)
                }
                ast::method_call(
                    None, // receiver_expr
                    bare_name_str,
                    args,
                    true,  // primary
                    false, // may_have_paren_wo_args
                )
            }
            _ => ast::bare_name(&bare_name_str)
        };
        self.lv -= 1;
        Ok(expr)
    }

    fn parse_const_ref(&mut self, s: String) -> Result<AstExpression, Error> {
        let mut names = vec![s];
        self.consume_token();
        // Parse `A::B`
        while self.current_token_is(Token::ColonColon) {
            self.consume_token();
            match self.current_token() {
                Token::UpperWord(s) => {
                    let name = s.to_string();
                    self.consume_token();
                    names.push(name);
                }
                token => return Err(parse_error!(self, "unexpected token: {:?}", token)),
            }
        }
        Ok(ast::const_ref(names))
    }

    fn parse_lambda(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_lambda");
        assert!(self.consume(Token::KwFn));
        self.expect(Token::LParen)?;
        let params = self.parse_params(vec![Token::RParen])?;
        self.skip_ws();
        self.expect(Token::LBrace)?;
        self.consume_token();
        let exprs = self.parse_exprs(vec![Token::RBrace])?;
        assert!(self.consume(Token::RBrace));
        self.lv -= 1;
        Ok(ast::lambda_expr(params, exprs))
    }

    fn parse_parenthesized_expr(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_parenthesized_expr");
        assert!(self.consume(Token::LParen));
        self.skip_wsn();
        let expr = self.parse_expr()?; // Should be parse_exprs() ?
        self.skip_wsn();
        self.expect(Token::RParen)?;
        self.lv -= 1;
        Ok(expr)
    }

    fn parse_array_literal(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_array_literal");
        assert!(self.consume(Token::LSqBracket));
        let mut exprs = vec![];
        self.skip_wsn();
        loop {
            match self.current_token() {
                Token::RSqBracket => {
                    self.consume_token();
                    break;
                }
                Token::Comma => {
                    return Err(parse_error!(self, "unexpected comma in an array literal"))
                }
                _ => {
                    let expr = self.parse_expr()?;
                    exprs.push(expr);
                    self.skip_wsn();
                    match self.current_token() {
                        Token::Comma => {
                            self.consume_token();
                            self.skip_wsn();
                        }
                        Token::RSqBracket => (),
                        token => {
                            return Err(parse_error!(
                                self,
                                "unexpected token `{:?}' in an array literal",
                                token
                            ))
                        }
                    }
                }
            }
        }
        self.lv -= 1;
        Ok(ast::array_literal(exprs))
    }

    fn parse_decimal_literal(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_decimal_literal");
        let expr = match self.consume_token() {
            Token::Number(s) => {
                if s.contains('.') {
                    let value = s.parse().unwrap();
                    ast::float_literal(value)
                } else {
                    let value = s.parse().unwrap();
                    ast::decimal_literal(value)
                }
            }
            _ => {
                self.lv -= 1;
                return Err(self.parseerror("expected decimal literal"));
            }
        };
        self.lv -= 1;
        Ok(expr)
    }

    fn parse_string(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_string");
        let expr = match self.consume_token() {
            Token::Str(content) => ast::string_literal(content),
            _ => panic!("parse_string called on non-string token"),
        };
        self.lv -= 1;
        Ok(expr)
    }

    // func: parse_xx
    // Parse `xx op xx op ... xx`
    fn parse_binary_operator<F: Fn(&mut Self) -> Result<AstExpression, Error>>(
        &mut self,
        name: &str,
        func: F,
        symbols: HashMap<Token, &str>,
    ) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log(name);
        let mut left = func(self)?;
        loop {
            let t = self.next_nonspace_token();
            let op = match symbols.get(&t) {
                Some(s) => s,
                None => {
                    self.lv -= 1;
                    return Ok(left);
                }
            };
            self.skip_ws();
            self.consume_token(); // Consume t
            self.skip_wsn(); // TODO: should ban ';' here
            let right = func(self)?;
            left = ast::bin_op_expr(left, op, right)
        }
    }

    /// Parse `do |..| ...end` or `{|..| ...}`, if any
    fn parse_opt_block(&mut self) -> Result<Option<AstExpression>, Error> {
        match self.next_nonspace_token() {
            Token::KwDo => {
                self.skip_ws();
                Ok(Some(self.parse_do_block()?))
            }
            Token::LBrace => {
                self.skip_ws();
                Ok(Some(self.parse_brace_block()?))
            }
            _ => Ok(None),
        }
    }

    /// Parse `do |..| ...end`, if any
    fn parse_opt_do_block(&mut self) -> Result<Option<AstExpression>, Error> {
        match self.current_token() {
            Token::KwDo => Ok(Some(self.parse_do_block()?)),
            _ => Ok(None),
        }
    }

    /// Parse `do |..| ...end`
    fn parse_do_block(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_do_block");
        self.expect(Token::KwDo)?;
        self.skip_ws();
        let block_params = if self.current_token_is(Token::Or) {
                               self.parse_block_params()?
                           } else {
                               vec![]
                           };
        self.skip_wsn();
        let body_exprs = self.parse_exprs(vec![Token::KwEnd])?;
        self.expect(Token::KwEnd)?;
        self.lv -= 1;
        Ok(ast::lambda_expr(block_params, body_exprs))
    }

    /// Parse `{|..| ...}`
    fn parse_brace_block(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_brace_block");
        self.expect(Token::LBrace)?;
        self.skip_ws();
        let block_params = if self.current_token_is(Token::Or) {
                               self.parse_block_params()?
                           } else {
                               vec![]
                           };
        self.skip_wsn();
        let body_exprs = self.parse_exprs(vec![Token::RBrace])?;
        self.expect(Token::RBrace)?;
        self.lv -= 1;
        Ok(ast::lambda_expr(block_params, body_exprs))
    }

    /// Parse `|a, b, ...|`
    fn parse_block_params(&mut self) -> Result<Vec<Param>, Error> {
        self.lv += 1;
        self.debug_log("parse_block_params");
        self.expect(Token::Or)?;
        self.skip_wsn();
        let params = self.parse_params(vec![Token::Or])?;
        self.lv -= 1;
        Ok(params)
    }
}
