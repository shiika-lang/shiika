use crate::base::*;
use crate::error::Error;
use crate::lexer::LexerState;
use shiika_ast::*;
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
                self.consume_token()?;
            } else if self.current_token_is(Token::Separator) {
                self.consume_token()?;
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
        let begin = self.lexer.location();
        let expr;
        if self.current_token_is(Token::KwVar) {
            self.consume_token()?;
            self.skip_ws()?;
            match self.current_token() {
                Token::LowerWord(s) => {
                    let name = s.to_string();
                    self.consume_token()?;
                    self.skip_ws()?;
                    self.expect(Token::Equal)?;
                    self.skip_wsn()?;
                    let rhs = self.parse_operator_expr()?;
                    let end = self.lexer.location();
                    expr = self.ast.lvar_decl(name, rhs, begin, end);
                }
                Token::IVar(s) => {
                    let name = s.to_string();
                    self.consume_token()?;
                    self.skip_ws()?;
                    self.expect(Token::Equal)?;
                    self.skip_wsn()?;
                    let rhs = self.parse_operator_expr()?;
                    let end = self.lexer.location();
                    expr = self.ast.ivar_decl(name, rhs, begin, end);
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
        let begin = self.lexer.location();
        let mut expr = self.parse_call_wo_paren()?;
        if self.next_nonspace_token()? == Token::ModIf {
            self.skip_ws()?;
            assert!(self.consume(Token::ModIf)?);
            self.skip_ws()?;
            let cond = self.parse_call_wo_paren()?;
            let end = self.lexer.location();
            expr = self.ast.if_expr(cond, vec![expr], None, begin, end)
        } else if self.next_nonspace_token()? == Token::ModUnless {
            self.skip_ws()?;
            assert!(self.consume(Token::ModUnless)?);
            self.skip_ws()?;
            let cond_inner = self.parse_call_wo_paren()?;
            let cond = self.ast.wrap_with_logical_not(cond_inner);
            let end = self.lexer.location();
            expr = self.ast.if_expr(cond, vec![expr], None, begin, end)
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
                if self.peek_next_token()? == Token::Space {
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
            let mut has_block = false;
            if !args.is_empty() {
                self.skip_ws()?;
                if let Some(lambda) = self.parse_opt_do_block()? {
                    args.push(lambda);
                    has_block = true;
                }
                expr = shiika_ast::set_method_call_args(expr, args, has_block);
            } else if self.next_nonspace_token()? == Token::KwDo {
                has_block = true;
                self.skip_ws()?;
                let lambda = self.parse_do_block()?;
                expr = shiika_ast::set_method_call_args(expr, vec![lambda], has_block);
            }
        }

        self.lv -= 1;
        Ok(expr)
    }

    // Returns `Some` if there is one of the following.
    // - `foo 1, 2, 3`
    // - `return 1`
    // Otherwise, returns `None` and rewind the lexer position.
    fn _try_parse_call_wo_paren(&mut self) -> Result<Option<AstExpression>, Error> {
        let begin = self.lexer.location();
        let first_token = self.current_token().clone();
        let cur = self.current_position();
        self.consume_token()?;
        self.set_lexer_state(LexerState::ExprArg);
        assert!(self.consume(Token::Space)?);
        let mut args = self.parse_operator_exprs()?;
        self.debug_log(&format!("tried/args: {:?}", args));
        if !args.is_empty() {
            self.skip_ws()?;
            let has_block = if let Some(lambda) = self.parse_opt_do_block()? {
                args.push(lambda);
                true
            } else {
                false
            };
            match &first_token {
                Token::LowerWord(s) => {
                    return Ok(Some(shiika_ast::method_call(
                        None,
                        s,
                        args,
                        vec![],
                        false,
                        has_block,
                        false,
                    )));
                }
                Token::KwReturn => {
                    if args.len() >= 2 {
                        return Err(parse_error!(
                            self,
                            "`return' cannot take more than one args"
                        ));
                    }
                    let end = self.lexer.location();
                    return Ok(Some(self.ast.return_expr(
                        Some(args.pop().unwrap()),
                        begin,
                        end,
                    )));
                }
                _ => panic!("must not happen: {:?}", self.current_token()),
            }
        }
        // Failed. Rollback the lexer changes
        self.rewind_to(cur)?;
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
        if self.next_nonspace_token()?.value_starts() {
            v.push(self.parse_operator_expr()?);
            loop {
                self.skip_ws()?;
                if !self.consume(Token::Comma)? {
                    break;
                }
                self.skip_wsn()?;
                v.push(self.parse_operator_expr()?);
            }
        }
        self.lv -= 1;
        Ok(v)
    }

    // operatorExpression:
    //   assignmentExpression |
    //   conditionalOperatorExpression (removed; next one is range_expr)
    fn parse_operator_expr(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_operator_expr");
        let mut expr = self.parse_range_expr()?;
        if expr.is_lhs() && self.next_nonspace_token()?.is_assignment_token() {
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

        self.skip_ws()?;
        let op = self.next_nonspace_token()?;
        self.consume_token()?;
        self.skip_wsn()?;
        let rhs = self.parse_operator_expr()?;

        self.lv -= 1;

        Ok(match op {
            Token::Equal => shiika_ast::assignment(lhs, rhs),
            Token::PlusEq => {
                shiika_ast::assignment(lhs.clone(), shiika_ast::bin_op_expr(lhs, "+", rhs))
            }
            Token::MinusEq => {
                shiika_ast::assignment(lhs.clone(), shiika_ast::bin_op_expr(lhs, "-", rhs))
            }
            Token::MulEq => {
                shiika_ast::assignment(lhs.clone(), shiika_ast::bin_op_expr(lhs, "*", rhs))
            }
            Token::DivEq => {
                shiika_ast::assignment(lhs.clone(), shiika_ast::bin_op_expr(lhs, "/", rhs))
            }
            Token::ModEq => {
                shiika_ast::assignment(lhs.clone(), shiika_ast::bin_op_expr(lhs, "%", rhs))
            }
            Token::LShiftEq => {
                shiika_ast::assignment(lhs.clone(), shiika_ast::bin_op_expr(lhs, "<<", rhs))
            }
            Token::RShiftEq => {
                shiika_ast::assignment(lhs.clone(), shiika_ast::bin_op_expr(lhs, ">>", rhs))
            }
            Token::AndEq => {
                shiika_ast::assignment(lhs.clone(), shiika_ast::bin_op_expr(lhs, "&", rhs))
            }
            Token::OrEq => {
                shiika_ast::assignment(lhs.clone(), shiika_ast::bin_op_expr(lhs, "|", rhs))
            }
            Token::XorEq => {
                shiika_ast::assignment(lhs.clone(), shiika_ast::bin_op_expr(lhs, "^", rhs))
            }
            _unexpected => unimplemented!(),
        })
    }

    // TODO: decide the symbol
    // Maybe `a..=b` and `a..<b` ?
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
        //                Ok(shiika_ast::range_expr(Some(expr), Some(end_expr), inclusive))
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
        let mut token = &self.next_nonspace_token()?;
        loop {
            if *token == Token::KwOr {
                self.skip_ws()?;
                assert!(self.consume(Token::KwOr)?);
                self.skip_wsn()?;
                let right_expr = self.parse_operator_and()?;
                expr = self.ast.logical_or(expr, right_expr);
                self.skip_ws()?;
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
        let mut token = &self.next_nonspace_token()?;
        loop {
            if *token == Token::KwAnd {
                self.skip_ws()?;
                assert!(self.consume(Token::KwAnd)?);
                self.skip_wsn()?;
                let right_expr = self.parse_equality_expr()?;
                expr = self.ast.logical_and(expr, right_expr);
                self.skip_ws()?;
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
        let begin = self.lexer.location();
        let left = self.parse_relational_expr()?;
        let op = match self.next_nonspace_token()? {
            Token::EqEq => "==",
            Token::NotEq => "!=",
            _ => {
                self.lv -= 1;
                return Ok(left);
            }
        };

        self.skip_ws()?;
        self.consume_token()?;
        self.skip_wsn()?;
        let right = self.parse_relational_expr()?;
        let end = self.lexer.location();
        let call_eq = self
            .ast
            .simple_method_call(Some(left), "==", vec![right], begin, end);
        let expr = if op == "!=" {
            self.ast.wrap_with_logical_not(call_eq)
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
        let mut begin = self.lexer.location();
        let mut expr = self.parse_bitwise_or()?; // additive (> >= < <=) additive
        let mut nesting = false;
        loop {
            let op = match self.next_nonspace_token()? {
                Token::LessThan => "<",
                Token::GreaterThan => ">",
                Token::LessEq => "<=",
                Token::GreaterEq => ">=",
                _ => break,
            };
            self.skip_ws()?;
            self.consume_token()?;
            self.skip_wsn()?;
            let right = self.parse_bitwise_or()?;
            let end = self.lexer.location();

            if nesting {
                if let AstExpressionBody::MethodCall { arg_exprs, .. } = &expr.body {
                    let mid = arg_exprs[0].clone();
                    let compare =
                        self.ast
                            .simple_method_call(Some(mid), op, vec![right], begin, end.clone());
                    expr = self.ast.logical_and(expr, compare);
                }
            } else {
                expr = self
                    .ast
                    .simple_method_call(Some(expr), op, vec![right], begin, end.clone());
                nesting = true;
            }
            begin = end;
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
        let expr = if self.consume(Token::UnaryMinus)? {
            let target = self.parse_unary_expr()?;
            shiika_ast::unary_expr(target, "-@")
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
        let begin = self.lexer.location();
        let expr = if self.consume(Token::KwNot)? {
            self.skip_ws()?;
            let target = self.parse_secondary_expr()?;
            let end = self.lexer.location();
            self.ast.logical_not(target, begin, end)
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
            Token::KwMatch => self.parse_match_expr(),
            Token::KwWhile => self.parse_while_expr(),
            _ => self.parse_primary_expr(),
        }?;
        self.lv -= 1;
        Ok(expr)
    }

    fn parse_break_expr(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_break_expr");
        let begin = self.lexer.location();
        assert!(self.consume(Token::KwBreak)?);
        self.lv -= 1;
        let end = self.lexer.location();
        Ok(self.ast.break_expr(begin, end))
    }

    fn parse_if_expr(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_if_expr");
        let begin = self.lexer.location();
        assert!(self.consume(Token::KwIf)?);
        self.skip_ws()?;
        // cond
        let cond_expr = self.parse_call_wo_paren()?;
        self.skip_ws()?;

        // `then`
        if self.consume(Token::KwThen)? {
            self.skip_wsn()?;
        } else {
            self.set_lexer_state(LexerState::ExprBegin); // +/- is always unary here
            self.expect(Token::Separator)?;
        }

        // then body
        let then_exprs = self.parse_exprs(vec![Token::KwEnd, Token::KwElse, Token::KwElsif])?;
        self.skip_wsn()?;

        self._parse_if_expr(cond_expr, then_exprs, begin)
    }

    /// Parse latter part of if-expr
    fn _parse_if_expr(
        &mut self,
        cond_expr: AstExpression,
        then_exprs: Vec<AstExpression>,
        begin: Location,
    ) -> Result<AstExpression, Error> {
        if self.consume(Token::KwElsif)? {
            self.skip_ws()?;
            let cond_expr2 = self.parse_expr()?;
            self.skip_ws()?;
            if self.consume(Token::KwThen)? {
                self.skip_wsn()?;
            } else {
                self.expect(Token::Separator)?;
            }
            let then_exprs2 =
                self.parse_exprs(vec![Token::KwEnd, Token::KwElse, Token::KwElsif])?;
            self.skip_wsn()?;
            let cont = self._parse_if_expr(cond_expr2, then_exprs2, begin.clone())?;
            let end = cont.locs.end.clone();
            Ok(self
                .ast
                .if_expr(cond_expr, then_exprs, Some(vec![cont]), begin, end))
        } else if self.consume(Token::KwElse)? {
            self.skip_wsn()?;
            let else_exprs = self.parse_exprs(vec![Token::KwEnd])?;
            self.skip_wsn()?;
            self.expect(Token::KwEnd)?;
            self.lv -= 1;
            let end = self.lexer.location();
            Ok(self
                .ast
                .if_expr(cond_expr, then_exprs, Some(else_exprs), begin, end))
        } else {
            self.expect(Token::KwEnd)?;
            self.lv -= 1;
            let end = self.lexer.location();
            Ok(self.ast.if_expr(cond_expr, then_exprs, None, begin, end))
        }
    }

    fn parse_unless_expr(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_unless_expr");
        let begin = self.lexer.location();
        assert!(self.consume(Token::KwUnless)?);
        self.skip_ws()?;
        let cond_expr = self.parse_call_wo_paren()?;
        self.skip_ws()?;
        if self.consume(Token::KwThen)? {
            self.skip_wsn()?;
        } else {
            self.expect(Token::Separator)?;
        }
        let then_exprs = self.parse_exprs(vec![Token::KwEnd, Token::KwElse])?;
        self.skip_wsn()?;
        if self.consume(Token::KwElse)? {
            return Err(parse_error!(self, "unless cannot have a else clause"));
        }
        self.expect(Token::KwEnd)?;
        self.lv -= 1;
        let end = self.lexer.location();
        Ok(self.ast.if_expr(
            self.ast.wrap_with_logical_not(cond_expr),
            then_exprs,
            None,
            begin,
            end,
        ))
    }

    fn parse_match_expr(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_match_expr");
        let begin = self.lexer.location();
        assert!(self.consume(Token::KwMatch)?);
        self.skip_ws()?;
        let cond_expr = self.parse_call_wo_paren()?;
        self.skip_wsn()?;

        let mut clauses = vec![];
        loop {
            match self.current_token() {
                Token::KwWhen => {
                    self.consume_token()?;
                    self.skip_ws()?;
                    let pattern = self.parse_pattern()?;
                    self.skip_ws()?;
                    if self.current_token_is(Token::KwThen) {
                        self.consume_token()?;
                    } else {
                        self.expect_sep()?;
                    }
                    let exprs =
                        self.parse_exprs(vec![Token::KwEnd, Token::KwWhen, Token::KwElse])?;
                    clauses.push((pattern, exprs));
                }
                Token::KwElse => {
                    self.consume_token()?;
                    let exprs = self.parse_exprs(vec![Token::KwEnd])?;
                    let pattern = shiika_ast::AstPattern::VariablePattern("_".to_string());
                    clauses.push((pattern, exprs));
                }
                Token::KwEnd => {
                    self.consume_token()?;
                    break;
                }
                token => {
                    return Err(parse_error!(
                        self,
                        "expected `when', `else' or `end' but got {:?}",
                        token
                    ));
                }
            }
        }
        self.lv -= 1;
        let end = self.lexer.location();
        Ok(self.ast.match_expr(cond_expr, clauses, begin, end))
    }

    fn parse_while_expr(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_while_expr");
        let begin = self.lexer.location();
        assert!(self.consume(Token::KwWhile)?);
        self.skip_ws()?;
        let cond_expr = self.parse_call_wo_paren()?;
        self.skip_ws()?;
        self.expect(Token::Separator)?;
        let body_exprs = self.parse_exprs(vec![Token::KwEnd])?;
        self.skip_wsn()?;
        self.expect(Token::KwEnd)?;
        self.lv -= 1;
        let end = self.lexer.location();
        Ok(self.ast.while_expr(cond_expr, body_exprs, begin, end))
    }

    // prim . methodName argumentWithParentheses? block?
    // prim [ indexingArgumentList? ] not(EQUAL)
    fn parse_primary_expr(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_primary_expr");
        let mut expr = self.parse_atomic()?;
        loop {
            if self.consume(Token::LSqBracket)? {
                let arg = self.parse_operator_expr()?;
                // TODO: parse multiple arguments
                self.skip_wsn()?;
                self.expect(Token::RSqBracket)?;
                expr = shiika_ast::method_call(
                    Some(expr),
                    "[]",
                    vec![arg],
                    vec![],
                    true,
                    false,
                    false,
                );
            } else if self.next_nonspace_token()? == Token::Dot {
                // TODO: Newline should also be allowed here (but Semicolon is not)
                self.skip_ws()?;
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
        self.set_lexer_state(LexerState::MethodName);
        assert!(self.consume(Token::Dot)?);
        self.set_lexer_state(LexerState::ExprEnd);
        self.skip_wsn()?;

        // Method name
        let method_name = self.get_method_name()?.to_string();
        self.consume_token()?;

        // Type args (Optional)
        let mut type_args = vec![];
        if self.current_token_is(Token::LessThan) {
            // TODO: Allow `ary.map< Int >{ ... }` ?
            if let Token::UpperWord(_) = self.peek_next_token()? {
                self.consume_token()?;
                type_args = self.parse_type_arguments()?;
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
        let has_block = if let Some(lambda) = self.parse_opt_block()? {
            args.push(lambda);
            true
        } else {
            false
        };

        self.lv -= 1;
        Ok(shiika_ast::method_call(
            Some(expr),
            &method_name,
            args,
            type_args,
            true,
            has_block,
            may_have_paren_wo_args,
        ))
    }

    fn parse_type_arguments(&mut self) -> Result<Vec<AstExpression>, Error> {
        self.lv += 1;
        self.debug_log("parse_type_arguments");
        let mut type_args = vec![];
        loop {
            type_args.push(self.parse_specialize_expression()?);
            self.skip_ws()?;
            match self.current_token() {
                Token::Comma => {
                    self.consume_token()?;
                    self.skip_wsn()?;
                    if let Token::UpperWord(_) = self.current_token() {
                        // Go next loop
                    } else {
                        return Err(parse_error!(
                            self,
                            "unexpected token in method call type arguments: {:?}",
                            self.current_token()
                        ));
                    }
                }
                Token::GreaterThan => {
                    self.consume_token()?;
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
        assert!(self.consume(Token::LParen)?);
        self.skip_wsn()?;
        let args;
        if self.consume(Token::RParen)? {
            args = vec![]
        } else {
            args = self.parse_operator_exprs()?;
            self.skip_wsn()?;
            self.expect(Token::RParen)?;
        }
        self.lv -= 1;
        Ok(args)
    }

    /// Smallest parts of Shiika program, such as number literals
    fn parse_atomic(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_atomic");
        let begin = self.lexer.location();
        let token = self.current_token();
        let expr = match token {
            Token::LowerWord(s) => {
                let name = s.to_string();
                self.consume_token()?;
                self.parse_primary_method_call(&name)
            }
            Token::KwReturn => {
                self.consume_token()?;
                let end = self.lexer.location();
                Ok(self.ast.return_expr(None, begin, end))
            }
            Token::UpperWord(_) => self.parse_specialize_expression(),
            Token::KwFn => self.parse_lambda(),
            Token::KwSelf | Token::KwTrue | Token::KwFalse => {
                let t = token.clone();
                self.consume_token()?;
                let end = self.lexer.location();
                Ok(self.ast.pseudo_variable(t, begin, end))
            }
            Token::IVar(s) => {
                let name = s.to_string();
                self.consume_token()?;
                let end = self.lexer.location();
                Ok(self.ast.ivar_ref(name, begin, end))
            }
            Token::LSqBracket => self.parse_array_literal(),
            Token::Number(_) => self.parse_decimal_literal(),
            Token::Str(_) => self.parse_string_literal(),
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
                let has_block = if let Some(lambda) = self.parse_opt_block()? {
                    args.push(lambda);
                    true
                } else {
                    false
                };
                shiika_ast::method_call(
                    None, // receiver_expr
                    bare_name_str,
                    args,
                    vec![], // TODO: type_args
                    true,   // primary
                    has_block,
                    false, // may_have_paren_wo_args
                )
            }
            _ => shiika_ast::bare_name(bare_name_str),
        };
        self.lv -= 1;
        Ok(expr)
    }

    /// Parse a constant name
    pub(super) fn parse_specialize_expression(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_specialize_expression");
        self.set_lexer_gtgt_mode(true); // Prevent `>>` is parsed as RShift
        let name = self._parse_specialize_expr()?;
        self.set_lexer_gtgt_mode(false); // End special mode
        self.lv -= 1;
        Ok(name)
    }

    /// Main routine of parse_specialize_expression
    fn _parse_specialize_expr(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("_parse_specialize_expr");
        let begin = self.lexer.location();
        let mut names = vec![];
        if let Token::UpperWord(s) = self.consume_token()? {
            names.push(s);
        } else {
            panic!("expected UpperWord");
        };
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
                    if lessthan_seen {
                        let inner = self._parse_specialize_expr()?;
                        args.push(inner);
                        self.skip_wsn()?;
                    } else {
                        let name = s.to_string();
                        self.consume_token()?;
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
        let end = self.lexer.location();
        self.lv -= 1;
        if args.is_empty() {
            Ok(self.ast.capitalized_name(names, begin, end))
        } else {
            Ok(self.ast.specialize_expr(names, args, begin, end))
        }
    }

    /// Parse `fn(){}`
    fn parse_lambda(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_lambda");
        assert!(self.consume(Token::KwFn)?);
        let params;
        if self.consume(Token::LParen)? {
            params = self.parse_block_params(true, &Token::RParen)?;
            self.skip_ws()?;
        } else {
            params = vec![];
        }
        self.skip_ws()?;
        self.expect(Token::LBrace)?;
        let exprs = self.parse_exprs(vec![Token::RBrace])?;
        assert!(self.consume(Token::RBrace)?);
        self.lv -= 1;
        Ok(shiika_ast::lambda_expr(params, exprs, true))
    }

    fn parse_parenthesized_expr(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_parenthesized_expr");
        assert!(self.consume(Token::LParen)?);
        self.skip_wsn()?;
        let expr = self.parse_expr()?; // Should be parse_exprs() ?
        self.skip_wsn()?;
        self.expect(Token::RParen)?;
        self.lv -= 1;
        Ok(expr)
    }

    fn parse_array_literal(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_array_literal");
        let begin = self.lexer.location();
        assert!(self.consume(Token::LSqBracket)?);
        let mut exprs = vec![];
        self.skip_wsn()?;
        loop {
            match self.current_token() {
                Token::RSqBracket => {
                    self.consume_token()?;
                    break;
                }
                Token::Comma => {
                    return Err(parse_error!(self, "unexpected comma in an array literal"))
                }
                _ => {
                    let expr = self.parse_call_wo_paren()?;
                    exprs.push(expr);
                    self.skip_wsn()?;
                    match self.current_token() {
                        Token::Comma => {
                            self.consume_token()?;
                            self.skip_wsn()?;
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
        let end = self.lexer.location();
        self.lv -= 1;
        Ok(self.ast.array_literal(exprs, begin, end))
    }

    fn parse_decimal_literal(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_decimal_literal");
        let begin = self.lexer.location();
        let expr = match self.consume_token()? {
            Token::Number(s) => {
                if s.contains('.') {
                    let end = self.lexer.location();
                    let value = s.parse().unwrap();
                    self.ast.float_literal(value, begin, end)
                } else {
                    let end = self.lexer.location();
                    let value = s.parse().unwrap();
                    self.ast.decimal_literal(value, begin, end)
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

    fn parse_string_literal(&mut self) -> Result<AstExpression, Error> {
        let begin = self.lexer.location();
        if let Token::Str(content) = self.consume_token()? {
            let end = self.lexer.location();
            Ok(self.ast.string_literal(content, begin, end))
        } else {
            Err(self.parseerror("invalid call"))
        }
    }

    /// Process a string literal with interpolation (`#{}`)
    fn parse_string_with_interpolation(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_string_with_interpolation");
        let mut begin = self.lexer.location();
        let (head, inspect1) =
            if let Token::StrWithInterpolation { head, inspect } = self.consume_token()? {
                (head, inspect)
            } else {
                panic!("invalid call")
            };
        let mut end = self.lexer.location();
        let mut inspect = inspect1;
        let mut expr = self.ast.string_literal(head, begin, end);
        begin = self.lexer.location();
        loop {
            self.skip_wsn()?;
            let inner_expr = self.parse_expr()?;
            end = self.lexer.location();
            let arg = self.ast.simple_method_call(
                Some(inner_expr),
                if inspect { "inspect" } else { "to_s" },
                vec![],
                begin.clone(),
                end.clone(),
            );
            expr = self
                .ast
                .simple_method_call(Some(expr), "+", vec![arg], begin, end);
            self.set_lexer_state(LexerState::StrLiteral);
            self.expect(Token::RBrace)?;
            self.set_lexer_state(LexerState::ExprEnd);
            begin = self.lexer.location();
            let (s, finish) = match self.consume_token()? {
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
            let end = self.lexer.location();
            expr = self.ast.simple_method_call(
                Some(expr),
                "+",
                vec![self.ast.string_literal(s, begin.clone(), end.clone())],
                begin.clone(),
                end,
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
            let t = self.next_nonspace_token()?;
            let op = match symbols.get(&t) {
                Some(s) => s,
                None => {
                    self.lv -= 1;
                    return Ok(left);
                }
            };
            self.skip_ws()?;
            self.consume_token()?; // Consume t
            self.skip_wsn()?; // TODO: should ban ';' here
            let right = func(self)?;
            left = shiika_ast::bin_op_expr(left, op, right)
        }
    }

    /// Parse `do |..| ...end` or `{|..| ...}`, if any
    fn parse_opt_block(&mut self) -> Result<Option<AstExpression>, Error> {
        match self.next_nonspace_token()? {
            Token::KwDo => {
                self.skip_ws()?;
                Ok(Some(self.parse_do_block()?))
            }
            Token::LBrace => {
                self.skip_ws()?;
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
        self.skip_ws()?;
        let block_params = if self.consume(Token::Or)? {
            self.parse_block_params(false, &Token::Or)?
        } else {
            vec![]
        };
        self.skip_wsn()?;
        let body_exprs = self.parse_exprs(vec![Token::KwEnd])?;
        self.expect(Token::KwEnd)?;
        self.lv -= 1;
        Ok(shiika_ast::lambda_expr(block_params, body_exprs, false))
    }

    /// Parse `{|..| ...}`
    fn parse_brace_block(&mut self) -> Result<AstExpression, Error> {
        self.lv += 1;
        self.debug_log("parse_brace_block");
        self.expect(Token::LBrace)?;
        self.skip_ws()?;
        let block_params = if self.consume(Token::Or)? {
            self.parse_block_params(false, &Token::Or)?
        } else {
            vec![]
        };
        self.skip_wsn()?;
        let body_exprs = self.parse_exprs(vec![Token::RBrace])?;
        self.expect(Token::RBrace)?;
        self.lv -= 1;
        Ok(shiika_ast::lambda_expr(block_params, body_exprs, false))
    }

    /// Parse `a, b, ...` in `|...|` or `fn(...){`
    fn parse_block_params(
        &mut self,
        type_required: bool,
        stop_tok: &Token,
    ) -> Result<Vec<BlockParam>, Error> {
        self.lv += 1;
        self.debug_log("parse_block_params");
        self.skip_ws()?;
        let mut params = vec![];
        let mut comma_seen = false;
        loop {
            match self.current_token() {
                Token::Comma => {
                    if comma_seen {
                        return Err(parse_error!(self, "extra comma in block params"));
                    } else {
                        self.consume_token()?;
                        self.skip_wsn()?;
                        comma_seen = true;
                    }
                }
                Token::LowerWord(_) => {
                    params.push(self.parse_block_param(type_required)?);
                    comma_seen = false;
                }
                token => {
                    if token == stop_tok {
                        if comma_seen {
                            return Err(parse_error!(self, "extra comma in block params"));
                        } else {
                            self.consume_token()?;
                            break;
                        }
                    }
                    return Err(parse_error!(
                        self,
                        "invalid token in block params: {:?}",
                        token
                    ));
                }
            }
        }
        self.lv -= 1;
        Ok(params)
    }

    fn parse_block_param(&mut self, type_required: bool) -> Result<BlockParam, Error> {
        // Name
        let name;
        match self.current_token() {
            Token::LowerWord(s) => {
                name = s.to_string();
                self.consume_token()?;
            }
            token => {
                return Err(parse_error!(
                    self,
                    "invalid token as block param: {:?}",
                    token
                ))
            }
        }
        self.skip_ws()?;

        // `:' Type
        let opt_typ = if self.current_token_is(Token::Colon) {
            self.consume_token()?;
            self.skip_ws()?;
            Some(self.parse_typ()?)
        } else {
            if type_required {
                return Err(parse_error!(
                    self,
                    "type annotation of fn param is mandatory"
                ));
            }
            None
        };

        Ok(shiika_ast::BlockParam { name, opt_typ })
    }

    /// Parse pattern of match expr
    fn parse_pattern(&mut self) -> Result<AstPattern, Error> {
        self.lv += 1;
        self.debug_log("parse_pattern");
        let token = self.current_token();
        let item = match token {
            Token::LowerWord(s) => {
                let name = s.to_string();
                self.consume_token()?;
                shiika_ast::AstPattern::VariablePattern(name)
            }
            Token::UpperWord(s) => {
                let name = s.to_string();
                self.consume_token()?;
                self.parse_extractor_pattern(name)?
            }
            Token::KwTrue | Token::KwFalse => {
                let b = *token == Token::KwTrue;
                self.consume_token()?;
                shiika_ast::AstPattern::BooleanLiteralPattern(b)
            }
            Token::Number(s) => {
                if s.contains('.') {
                    let value = s.parse().unwrap();
                    self.consume_token()?;
                    shiika_ast::AstPattern::FloatLiteralPattern(value)
                } else {
                    let value = s.parse().unwrap();
                    self.consume_token()?;
                    shiika_ast::AstPattern::IntegerLiteralPattern(value)
                }
            }
            Token::Str(content) => {
                let s = content.to_string();
                self.consume_token()?;
                shiika_ast::AstPattern::StringLiteralPattern(s)
            }
            Token::StrWithInterpolation { .. } => {
                todo!()
            }
            _ => {
                return Err(parse_error!(self, "expected a pattern but got {:?}", token));
            }
        };
        self.lv -= 1;
        Ok(item)
    }

    /// Parse pattern like `Some(val)`
    fn parse_extractor_pattern(&mut self, upper_word: String) -> Result<AstPattern, Error> {
        self.lv += 1;
        self.debug_log("parse_extractor_pattern");

        // Class name
        let mut names = vec![upper_word];
        loop {
            if self.consume(Token::ColonColon)? {
                let token = self.current_token();
                if let Token::UpperWord(s) = token {
                    names.push(s.to_string());
                    self.consume_token()?;
                } else {
                    return Err(parse_error!(self, "unexpected token: {:?}", token));
                }
            } else {
                break;
            }
        }

        // Parameters (optional)
        let mut params = vec![];
        if self.consume(Token::LParen)? {
            self.skip_wsn()?;
            loop {
                if self.consume(Token::RParen)? {
                    break;
                }
                if !params.is_empty() {
                    self.expect(Token::Comma)?;
                    self.skip_wsn()?;
                }
                params.push(self.parse_pattern()?);
                self.skip_wsn()?;
            }
        }

        self.lv -= 1;
        Ok(shiika_ast::AstPattern::ExtractorPattern { names, params })
    }
}
