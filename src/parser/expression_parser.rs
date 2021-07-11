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
            expr = self.parse_if_unless_modifier()?;
        }
        self.lv -= 1;
        Ok(expr)
    }

    /// a if b
    /// a unless b
    pub fn parse_if_unless_modifier(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_if_unless_modifier");
        let mut expr = self.parse_call_wo_paren()?;
        if self.next_nonspace_token() == Token::ModIf {
            self.skip_ws();
            assert!(self.consume(Token::ModIf));
            self.skip_ws();
            let cond = self.parse_call_wo_paren()?;
            expr = ast::if_expr(cond, vec![expr], None)
        } else if self.next_nonspace_token() == Token::ModUnless {
            self.skip_ws();
            assert!(self.consume(Token::ModUnless));
            self.skip_ws();
            let cond = ast::logical_not(self.parse_call_wo_paren()?);
            expr = ast::if_expr(cond, vec![expr], None)
        }
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
        match &self.current_token() {
            Token::LowerWord(_) | Token::KwReturn => {
                if self.peek_next_token() == Token::Space {
                    if let Some(expr) = self._try_parse_call_wo_paren()? {
                        self.lv -= 1;
                        return Ok(expr);
                    }
                }
            }
            _ => (),
        }

        // If not, read an expression
        let mut expr = self.parse_operator_expr()?;

        // See if it is a method invocation (eg. `x.foo 1, 2`)
        if expr.may_have_paren_wo_args() {
            let mut args = self.parse_operator_exprs()?;
            if !args.is_empty() {
                self.skip_ws();
                if let Some(lambda) = self.parse_opt_do_block()? {
                    args.push(lambda);
                }
                expr = ast::set_method_call_args(expr, args);
            } else if self.next_nonspace_token() == Token::KwDo {
                self.skip_ws();
                let lambda = self.parse_do_block()?;
                expr = ast::set_method_call_args(expr, vec![lambda]);
            }
        }

        self.lv -= 1;
        Ok(expr)
    }

    // Returns `Some` if there is one of the following.
    // Otherwise, returns `None` and rewind the lexer position.
    // - `foo 1, 2, 3`
    // - `return 1`
    fn _try_parse_call_wo_paren(&mut self) -> Result<Option<AstExpression>, Error> {
        let token = self.current_token().clone();
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
            match &token {
                Token::LowerWord(s) => {
                    return Ok(Some(ast::method_call(None, s, args, vec![], false, false)));
                }
                Token::KwReturn => {
                    if args.len() >= 2 {
                        return Err(parse_error!(
                            self,
                            "`return' cannot take more than one args"
                        ));
                    }
                    return Ok(Some(ast::return_expr(Some(args.pop().unwrap()))));
                }
                _ => panic!("must not happen: {:?}", self.current_token()),
            }
        }
        // Failed. Rollback the lexer changes
        self.rewind_to(cur);
        self.set_lexer_state(LexerState::ExprArg);
        Ok(None)
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
        let mut expr = self.parse_conditional_expr()?;
        if expr.is_lhs() && self.next_nonspace_token().is_assignment_token() {
            expr = self.parse_assignment_expr(expr)?;
        }
        self.lv -= 1;
        Ok(expr)
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

    /// `or`
    fn parse_operator_or(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_operator_or");
        let mut expr = self.parse_operator_and()?;
        let mut token = &self.next_nonspace_token();
        loop {
            if *token == Token::KwOr {
                self.skip_ws();
                assert!(self.consume(Token::KwOr));
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

    /// `and`
    fn parse_operator_and(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_operator_and");
        let mut expr = self.parse_equality_expr()?;
        let mut token = &self.next_nonspace_token();
        loop {
            if *token == Token::KwAnd {
                self.skip_ws();
                assert!(self.consume(Token::KwAnd));
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
        let call_eq = ast::method_call(Some(left), "==", vec![right], vec![], false, false);
        let expr = if op == "!=" {
            ast::logical_not(call_eq)
        } else {
            call_eq
        };
        self.lv -= 1;
        Ok(expr)
    }

    /// <=, etc.
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
                    let compare =
                        ast::method_call(Some(mid), op, vec![right], vec![], false, false);
                    expr = ast::logical_and(expr, compare);
                }
            } else {
                expr = ast::method_call(Some(expr), op, vec![right], vec![], false, false);
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
        let expr = if self.consume(Token::UnaryMinus) {
            let target = self.parse_unary_expr()?;
            ast::unary_expr(target, "-@")
        } else {
            self.parse_unary_expr()?
        };
        self.lv -= 1;
        Ok(expr)
    }

    // TODO: Parse ~, +
    fn parse_unary_expr(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_unary_expr");
        let expr = if self.consume(Token::Bang) {
            let target = self.parse_secondary_expr()?;
            ast::logical_not(target)
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
            Token::KwBreak => Ok(self.parse_break_expr()),
            Token::KwIf => self.parse_if_expr(),
            Token::KwUnless => self.parse_unless_expr(),
            Token::KwWhile => self.parse_while_expr(),
            _ => self.parse_primary_expr(),
        }?;
        self.lv -= 1;
        Ok(expr)
    }

    fn parse_break_expr(&mut self) -> AstExpression {
        self.lv += 1;
        self.debug_log("parse_break_expr");
        assert!(self.consume(Token::KwBreak));
        self.lv -= 1;
        ast::break_expr()
    }

    fn parse_if_expr(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_if_expr");
        assert!(self.consume(Token::KwIf));
        self.skip_ws();

        // cond
        let cond_expr = self.parse_call_wo_paren()?;
        self.skip_ws();

        // `then`
        if self.consume(Token::KwThen) {
            self.skip_wsn();
        } else {
            self.set_lexer_state(LexerState::ExprBegin); // +/- is always unary here
            self.expect(Token::Separator)?;
        }

        // then body
        let then_exprs = self.parse_exprs(vec![Token::KwEnd, Token::KwElse, Token::KwElsif])?;
        self.skip_wsn();

        self._parse_if_expr(cond_expr, then_exprs)
    }

    /// Parse latter part of if-expr
    fn _parse_if_expr(
        &mut self,
        cond_expr: AstExpression,
        then_exprs: Vec<AstExpression>,
    ) -> Result<AstExpression, Error> {
        if self.consume(Token::KwElsif) {
            self.skip_ws();
            let cond_expr2 = self.parse_expr()?;
            self.skip_ws();
            if self.consume(Token::KwThen) {
                self.skip_wsn();
            } else {
                self.expect(Token::Separator)?;
            }
            let then_exprs2 =
                self.parse_exprs(vec![Token::KwEnd, Token::KwElse, Token::KwElsif])?;
            self.skip_wsn();
            Ok(ast::if_expr(
                cond_expr,
                then_exprs,
                Some(vec![self._parse_if_expr(cond_expr2, then_exprs2)?]),
            ))
        } else if self.consume(Token::KwElse) {
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
        self.debug_log("parse_unless_expr");
        assert!(self.consume(Token::KwUnless));
        self.skip_ws();
        let cond_expr = self.parse_call_wo_paren()?;
        self.skip_ws();
        if self.consume(Token::KwThen) {
            self.skip_wsn();
        } else {
            self.expect(Token::Separator)?;
        }
        let then_exprs = self.parse_exprs(vec![Token::KwEnd, Token::KwElse])?;
        self.skip_wsn();
        if self.consume(Token::KwElse) {
            return Err(parse_error!(self, "unless cannot have a else clause"));
        }
        self.expect(Token::KwEnd)?;
        self.lv -= 1;
        Ok(ast::if_expr(ast::logical_not(cond_expr), then_exprs, None))
    }

    fn parse_while_expr(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_while_expr");
        assert!(self.consume(Token::KwWhile));
        self.skip_ws();
        let cond_expr = self.parse_call_wo_paren()?;
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
            if self.consume(Token::LSqBracket) {
                let arg = self.parse_operator_expr()?;
                // TODO: parse multiple arguments
                self.skip_wsn();
                self.expect(Token::RSqBracket)?;
                expr = ast::method_call(Some(expr), "[]", vec![arg], vec![], true, false);
            } else if self.next_nonspace_token() == Token::Dot {
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

        // Type args (Optional)
        let mut type_args = vec![];
        if self.current_token_is(Token::LessThan) {
            // TODO: Allow `ary.map< Int >{ ... }` ?
            if let Token::UpperWord(s) = self.peek_next_token() {
                self.consume_token();
                self.consume_token();
                type_args = self.parse_type_arguments(s)?;
            }
        }

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
            type_args,
            true,
            may_have_paren_wo_args,
        ))
    }

    fn parse_type_arguments(&mut self, s: String) -> Result<Vec<AstExpression>, Error> {
        self.lv += 1;
        self.debug_log("parse_type_arguments");
        let mut name = s;
        let mut type_args = vec![];
        loop {
            type_args.push(self.parse_specialize_expression(name)?);
            self.skip_ws();
            match self.current_token() {
                Token::Comma => {
                    self.consume_token();
                    self.skip_wsn();
                    if let Token::UpperWord(s) = self.peek_next_token() {
                        name = s
                    } else {
                        return Err(parse_error!(
                            self,
                            "unexpected token in method call type arguments: {:?}",
                            self.current_token()
                        ));
                    }
                }
                Token::GreaterThan => {
                    self.consume_token();
                    break;
                }
                token => {
                    return Err(parse_error!(
                        self,
                        "unexpected token in method call type arguments: {:?}",
                        token
                    ));
                }
            }
        }
        self.lv -= 1;
        Ok(type_args)
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
            args = self.parse_operator_exprs()?;
            self.skip_wsn();
            self.expect(Token::RParen)?;
        }
        self.lv -= 1;
        Ok(args)
    }

    /// Smallest parts of Shiika program, such as number literals
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
            Token::KwReturn => {
                self.consume_token();
                Ok(ast::return_expr(None))
            }
            Token::UpperWord(s) => {
                let name = s.to_string();
                self.consume_token();
                self.parse_specialize_expression(name)
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
            Token::Str(_) => Ok(self.parse_string_literal()),
            Token::StrWithInterpolation { .. } => self.parse_string_with_interpolation(),
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
                    vec![], // TODO: type_args
                    true,   // primary
                    false,  // may_have_paren_wo_args
                )
            }
            _ => ast::bare_name(bare_name_str),
        };
        self.lv -= 1;
        Ok(expr)
    }

    /// Parse a constant name. `s` must be consumed beforehand
    pub(super) fn parse_specialize_expression(
        &mut self,
        s: String,
    ) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_specialize_expression");
        self.set_lexer_gtgt_mode(true); // Prevent `>>` is parsed as RShift
        let name = self._parse_specialize_expr(s)?;
        self.set_lexer_gtgt_mode(false); // End special mode
        self.lv -= 1;
        Ok(name)
    }

    /// Main routine of parse_specialize_expression
    fn _parse_specialize_expr(&mut self, s: String) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("_parse_const_ref");
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
                    self.consume_token();
                }
                Token::LessThan => {
                    // `A<B>`
                    lessthan_seen = true;
                    self.consume_token();
                    self.skip_wsn();
                }
                Token::GreaterThan => {
                    // `A<B>`
                    if lessthan_seen {
                        self.consume_token();
                    }
                    break;
                }
                Token::Comma => {
                    // `A<B, C>`
                    if lessthan_seen {
                        self.consume_token();
                        self.skip_wsn();
                    } else {
                        break;
                    }
                }
                Token::UpperWord(s) => {
                    let name = s.to_string();
                    self.consume_token();
                    if lessthan_seen {
                        let inner = self._parse_specialize_expr(name)?;
                        args.push(inner);
                        self.skip_wsn();
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
        if args.is_empty() {
            Ok(ast::const_ref(names))
        } else {
            Ok(ast::specialize_expr(names, args))
        }
    }

    /// Parse `fn(){}`
    fn parse_lambda(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_lambda");
        assert!(self.consume(Token::KwFn));
        let params;
        if self.consume(Token::LParen) {
            params = self.parse_params(false, vec![Token::RParen])?;
            self.skip_ws();
        } else {
            params = vec![];
        }
        self.skip_ws();
        self.expect(Token::LBrace)?;
        let exprs = self.parse_exprs(vec![Token::RBrace])?;
        assert!(self.consume(Token::RBrace));
        self.lv -= 1;
        Ok(ast::lambda_expr(params, exprs, true))
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
                    let expr = self.parse_call_wo_paren()?;
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

    fn parse_string_literal(&mut self) -> AstExpression {
        if let Token::Str(content) = self.consume_token() {
            ast::string_literal(content)
        } else {
            panic!("invalid call")
        }
    }

    /// Process a string literal with interpolation (`#{}`)
    fn parse_string_with_interpolation(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_string_with_interpolation");
        let (head, inspect1) =
            if let Token::StrWithInterpolation { head, inspect } = self.consume_token() {
                (head, inspect)
            } else {
                panic!("invalid call")
            };
        let mut inspect = inspect1;
        let mut expr = ast::string_literal(head);
        loop {
            self.skip_wsn();
            let inner_expr = self.parse_expr()?;
            let arg = ast::method_call(
                Some(inner_expr),
                if inspect { "inspect" } else { "to_s" },
                vec![],
                vec![],
                false, // primary
                false, // may_have_paren_wo_args
            );
            expr = ast::method_call(
                Some(expr),
                "+",
                vec![arg],
                vec![],
                false, // primary
                false, // may_have_paren_wo_args
            );
            self.set_lexer_state(LexerState::StrLiteral);
            self.expect(Token::RBrace)?;
            self.set_lexer_state(LexerState::ExprEnd);
            let (s, finish) = match self.consume_token() {
                Token::Str(tail) => (tail, true),
                Token::StrWithInterpolation {
                    head,
                    inspect: inspect2,
                } => {
                    inspect = inspect2;
                    (head, false)
                }
                _ => panic!("unexpeced token in LexerState::StrLiteral"),
            };
            expr = ast::method_call(
                Some(expr),
                "+",
                vec![ast::string_literal(s)],
                vec![],
                false, // primary
                false, // may_have_paren_wo_args
            );
            if finish {
                break;
            };
        }
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
        Ok(ast::lambda_expr(block_params, body_exprs, false))
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
        Ok(ast::lambda_expr(block_params, body_exprs, false))
    }

    /// Parse `|a, b, ...|`
    fn parse_block_params(&mut self) -> Result<Vec<Param>, Error> {
        self.lv += 1;
        self.debug_log("parse_block_params");
        self.expect(Token::Or)?;
        self.skip_wsn();
        let params = self.parse_params(false, vec![Token::Or])?;
        self.lv -= 1;
        Ok(params)
    }
}
