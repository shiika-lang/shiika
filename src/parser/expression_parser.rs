use crate::parser::base::*;
use crate::ast::ExpressionBody::*;

impl<'a> Parser<'a> {
    pub fn parse_exprs(&mut self) -> Result<Vec<ast::Expression>, Error> {
        let mut ret = Vec::new();
        loop {
            match self.current_token() {
                Token::Eof | Token::KwEnd => break,
                _ => ret.push(self.parse_expr()?),
            };
            self.expect_sep()?;
        }
        Ok(ret)
    }

    pub fn parse_expr(&mut self) -> Result<ast::Expression, Error> {
        self.lv += 1; self.debug_log("parse_expr");
        let mut expr = self.parse_not_expr()?;
        self.skip_ws();
        loop {
            match self.current_token() {
                Token::KwAnd => {
                    self.consume_token();
                    self.skip_wsn();
                    expr = ast::logical_and(expr, self.parse_not_expr()?);
                },
                Token::KwOr => {
                    self.consume_token();
                    self.skip_wsn();
                    expr = ast::logical_or(expr, self.parse_not_expr()?);
                },
                _ => break,
            }
            self.skip_ws();
        }
        self.lv -= 1;
        Ok(expr)
    }

    fn parse_not_expr(&mut self) -> Result<ast::Expression, Error> {
        self.lv += 1; self.debug_log("parse_not_expr");
        match self.current_token() {
            Token::KwOr => {
                self.skip_ws();
                let expr = self.parse_not_expr()?;
                self.lv -= 1;
                Ok(ast::logical_not(expr))
            },
            Token::Bang => {
                self.skip_ws();
                let mut expr = self.parse_operator_expr()?;
                expr = self.parse_call_wo_paren(expr)?;
                self.lv -= 1;
                Ok(ast::logical_not(expr))
            },
            _ => {
                let expr = self.parse_operator_expr()?;
                self.lv -= 1;
                Ok(self.parse_call_wo_paren(expr)?)
            }
        }
    }

    //        methodInvocationWithoutParentheses |
    //                MethodIdentifier bar (do ... end)?
    //                primaryExpression . foo bar (do ... end)?
    fn parse_call_wo_paren(&mut self, expr: ast::Expression) -> Result<ast::Expression, Error> {
        self.lv += 1; self.debug_log("parse_call_wo_paren");
        let expr = self.parse_method_chains(expr)?;
        self.lv -= 1;
        Ok(expr)
    }

//    // command:
//    //   MethodIdentifier ~ not(not(WS) ~ LPAREN) ~ argumentWithoutParentheses
//    //   primaryExpression ~ not(LineTerminator) ~ PERIOD ~ methodName ~ not(not(WS) ~ LPAREN) ~ argumentWithoutParentheses
//    fn parse_command(&mut self, mut expr: ast::Expression) -> Result<ast::Expression, Error> {
//    }

    /// Parse `foo.bar(args).baz quux`
    fn parse_method_chains(&mut self, mut expr: ast::Expression) -> Result<ast::Expression, Error> {
        self.lv += 1; self.debug_log(&format!("parse_method_chains(expr: {:?})", expr.body));
        // Check expr
        if let BareName(_) = expr.body { }
        else if expr.primary { }
        else { self.lv -= 1; return Ok(expr) }

        while expr.primary || expr.may_have_paren_wo_args() {
            if expr.may_have_paren_wo_args() && self.current_token_is(Token::Space) {
                self.skip_ws();
                if !self.current_token_is(Token::Separator) &&
                   !self.current_token_is(Token::LBrace) {
                    let args = self.parse_args()?; // TODO: do...end
                    expr = ast::set_method_call_args(expr, args);
                }
            }
            if expr.primary && self.next_nonspace_token() == Token::Dot {
                self.skip_ws();
                expr = self.parse_method_chain(expr)?;
            }
            else {
                break
            }
        }
        self.lv -= 1;
        Ok(expr)
    }

    /// Parse `.foo(args)`
    fn parse_method_chain(&mut self, expr: ast::Expression) -> Result<ast::Expression, Error> {
        self.lv += 1; self.debug_log("parse_method_chain");
        // .
        assert!(self.consume(Token::Dot));
        self.skip_wsn();

        // Method name
        let method_name = match self.current_token() {
            Token::LowerWord(s) => s.clone(),
            token => return Err(parse_error!(self, "invalid method name: {:?}", token))
        };
        self.consume_token();

        // Args
        let (args, may_have_paren_wo_args) = match self.current_token() {
            // .foo(args)
            Token::LParen => (self.parse_paren_and_args()?, false),
            // .foo
            _ => (vec![], true),
        };

        self.lv -= 1;
        Ok(ast::method_call(
                Some(expr),
                &method_name,
                args,
                true,
                may_have_paren_wo_args))
    }

    fn parse_paren_and_args(&mut self) -> Result<Vec<ast::Expression>, Error> {
        self.lv += 1; self.debug_log("parse_paren_and_args");
        assert!(self.consume(Token::LParen));
        self.skip_wsn();
        let args;
        if self.consume(Token::RParen) {
            args = vec![]
        }
        else {
            args = self.parse_args()?;
            self.skip_wsn();
            self.expect(Token::RParen)?;
        }
        self.lv -= 1;
        Ok(args)
    }

    fn parse_args(&mut self) -> Result<Vec<ast::Expression>, Error> {
        self.lv += 1; self.debug_log("parse_args");
        let expr = self.parse_operator_exprs()?;
        self.lv -= 1;
        Ok(expr)
    }

    fn parse_operator_exprs(&mut self) -> Result<Vec<ast::Expression>, Error> {
        self.lv += 1; self.debug_log("parse_operator_exprs");
        let mut v = vec![
            self.parse_operator_expr()?
        ];
        loop {
            self.skip_ws();
            if !self.current_token_is(Token::Comma) { break }
            self.consume_token();
            self.skip_wsn();
            v.push( self.parse_operator_expr()? );
        }
        self.lv -= 1;
        Ok(v)
    }

    fn parse_operator_expr(&mut self) -> Result<ast::Expression, Error> {
        self.lv += 1; self.debug_log("parse_operator_expr");
        let expr = self.parse_conditional_expr()?;
        if let BareName(_) = expr.body {
            // TODO: may be a singleVariableAssignmentExpression
            self.lv -= 1;
            Ok(expr)
        }
        else if expr.primary {
            // TODO: may be a singleIndexingAssignmentExpression or a singleMethodAssignmentExpression
            self.lv -= 1;
            Ok(expr)
        }
        else {
            self.lv -= 1;
            Ok(expr)
        }
    }

    /// `a ? b : c`
    fn parse_conditional_expr(&mut self) -> Result<ast::Expression, Error> {
        self.lv += 1; self.debug_log("parse_conditional_expr");
        let expr = self.parse_range_expr()?;
        if self.next_nonspace_token() == Token::Question {
            self.skip_ws(); assert!(self.consume(Token::Question));
            self.skip_wsn();
            let then_expr = self.parse_operator_expr()?;
            self.skip_ws();
            self.expect(Token::Colon)?;
            self.skip_wsn();
            let else_expr = self.parse_operator_expr()?;
            self.lv -= 1;
            Ok(ast::if_expr(expr, then_expr, Some(else_expr)))
        }
        else {
            self.lv -= 1;
            Ok(expr)
        }
    }

    /// `a..b`, `a...b`
    fn parse_range_expr(&mut self) -> Result<ast::Expression, Error> {
        self.lv += 1; self.debug_log("parse_range_expr");
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
    fn parse_operator_or(&mut self) -> Result<ast::Expression, Error> {
        self.lv += 1; self.debug_log("parse_operator_or");
        let mut expr = self.parse_operator_and()?;
        let mut token = &self.next_nonspace_token();
        loop {
            if *token == Token::OrOr {
                self.skip_ws(); assert!(self.consume(Token::OrOr));
                self.skip_wsn();
                expr = ast::logical_or(expr, self.parse_operator_and()?);
                self.skip_ws();
                token = self.current_token();
            }
            else {
                break
            }
        }
        self.lv -= 1;
        Ok(expr)
    }

    /// `&&`
    fn parse_operator_and(&mut self) -> Result<ast::Expression, Error> {
        self.lv += 1; self.debug_log("parse_operator_and");
        let mut expr = self.parse_equality_expr()?;
        let mut token = &self.next_nonspace_token();
        loop {
            if *token == Token::AndAnd {
                self.skip_ws(); assert!(self.consume(Token::AndAnd));
                self.skip_wsn();
                expr = ast::logical_and(expr, self.parse_equality_expr()?);
                self.skip_ws();
                token = self.current_token();
            }
            else {
                break
            }
        }
        self.lv -= 1;
        Ok(expr)
    }

    /// `==`, etc.
    fn parse_equality_expr(&mut self) -> Result<ast::Expression, Error> {
        self.lv += 1; self.debug_log("parse_equality_expr");
        //TODO:
        //  parse_relational_expr
        //  parse_bitwise_or
        //  parse_bitwise_and
        //  parse_bitwise_shift
        //  parse_additive_expr
        let expr = self.parse_additive_expr()?;
        self.lv -= 1;
        Ok(expr)
    }

    fn parse_additive_expr(&mut self) -> Result<ast::Expression, Error> {
        self.lv += 1; self.debug_log("parse_additive_expr");
        let left = self.parse_multiplicative_expr()?;

        let op = match self.next_nonspace_token() {
            Token::Plus => "+",
            Token::Minus => "-",
            _ => { self.lv -= 1; return Ok(left) },
        };
        self.skip_ws(); self.consume_token();
        self.skip_wsn();
        let right = self.parse_multiplicative_expr()?;
        self.lv -= 1;
        Ok(ast::bin_op_expr(left, &op, right))
    }

    fn parse_multiplicative_expr(&mut self) -> Result<ast::Expression, Error> {
        self.lv += 1; self.debug_log("parse_multiplicative_expr");
        //TODO:
        //  parse_multiplicative_expr
        //  parse_unary_minus_expr
        //  parse_power_expr
        //  parse_unary_expr
        //  parse_secondary_expr
        let left = self.parse_secondary_expr()?;

        let op = match self.next_nonspace_token() {
            Token::Mul => "*",
            Token::Div => "/",
            Token::Mod => "%",
            _ => { self.lv -= 1; return Ok(left) },
        };
        self.skip_ws(); self.consume_token();
        self.skip_wsn();
        let right = self.parse_secondary_expr()?;
        self.lv -= 1;
        Ok(ast::bin_op_expr(left, &op, right))
    }

    /// Secondary expression
    ///
    /// Mostly primary but cannot be a method receiver
    /// eg. 
    ///    NG: if foo then bar else baz end.quux()
    ///    OK: (if foo then bar else baz end).quux()
    fn parse_secondary_expr(&mut self) -> Result<ast::Expression, Error> {
        self.lv += 1; self.debug_log("parse_secondary_expr");
        let expr = match self.current_token() {
            Token::KwIf => self.parse_if_expr(),
            _ => self.parse_primary_expr()
        }?;
        self.lv -= 1;
        Ok(expr)
    }

    fn parse_if_expr(&mut self) -> Result<ast::Expression, Error> {
        self.lv += 1; self.debug_log("parse_if_expr");
        assert!(self.consume(Token::KwIf));
        self.skip_ws();
        let cond_expr = self.parse_expr()?;
        self.skip_ws();
        if self.consume(Token::KwThen) {
            self.skip_wsn();
        }
        else {
            self.expect(Token::Separator)?;
        }
        let then_expr = self.parse_expr()?;
        self.skip_wsn();
        if self.consume(Token::KwElse) {
            self.skip_wsn();
            let else_expr = Some(self.parse_expr()?);
            self.lv -= 1;
            Ok(ast::if_expr(cond_expr, then_expr, else_expr))
        }
        else {
            self.expect(Token::KwEnd)?;
            let else_expr = None;
            self.lv -= 1;
            Ok(ast::if_expr(cond_expr, then_expr, else_expr))
        }
    }

    // prim . methodName argumentWithParentheses? block?
    // prim [ indexingArgumentList? ] not(EQUAL)
    fn parse_primary_expr(&mut self) -> Result<ast::Expression, Error> {
        self.lv += 1; self.debug_log("parse_primary_expr");
        let mut expr = self.parse_atomic()?;
        loop {
            if self.next_nonspace_token() == Token::Dot { // TODO: Newline should also be allowed here (but Semicolon is not)
                self.skip_ws();
                expr = self.parse_method_chain(expr)?;
            }
            else {
                break
            }
        }
        self.lv -= 1;
        Ok(expr)
    }

    fn parse_atomic(&mut self) -> Result<ast::Expression, Error> {
        self.lv += 1; self.debug_log("parse_atomic");
        let token = self.current_token();
        let expr = match token {
            Token::LowerWord(s) => {
                let name = s.to_string();
                self.consume_token();
                self.parse_primary_method_call(&name)
            },
            Token::UpperWord(s) => {
                let expr = ast::const_ref(s);
                self.consume_token();
                Ok(expr)
            },
            Token::KwSelf | Token::KwTrue | Token::KwFalse => {
                let t = token.clone();
                self.consume_token();
                Ok(ast::pseudo_variable(t))
            },
            Token::Number(_) => {
                self.parse_decimal_literal()
            },
            Token::LParen => {
                self.parse_parenthesized_expr()
            },
            token => {
                Err(parse_error!(self, "unexpected token: {:?}", token))
            }
        }?;
        self.lv -= 1;
        Ok(expr)
    }

    // Method call with explicit parenthesis (eg. `foo(bar)`)
    fn parse_primary_method_call(&mut self, bare_name_str: &str) -> Result<ast::Expression, Error> {
        self.lv += 1; self.debug_log("parse_primary_method_call");
        let expr = match self.current_token() {
            Token::LParen => {
                let arg_exprs = self.parse_paren_and_args()?;
                ast::method_call(
                    None, // receiver_expr
                    bare_name_str,
                    arg_exprs,
                    true, // primary
                    false, // may_have_paren_wo_args
                )
            },
            _ => ast::bare_name(&bare_name_str)
        };
        self.lv -= 1;
        Ok(expr)
    }

    fn parse_parenthesized_expr(&mut self) -> Result<ast::Expression, Error> {
        self.lv += 1; self.debug_log("parse_parenthesized_expr");
        assert!(self.consume(Token::LParen));
        self.skip_wsn();
        let expr = self.parse_expr()?; // Should be parse_stmts() ?
        self.skip_wsn();
        self.expect(Token::RParen)?;
        self.lv -= 1;
        Ok(expr)
    }

    fn parse_decimal_literal(&mut self) -> Result<ast::Expression, Error> {
        self.lv += 1; self.debug_log("parse_parenthesized_expr");
        let expr = match self.consume_token() {
            Token::Number(s) => {
                if s.contains('.') {
                    let value = s.parse().unwrap();
                    ast::float_literal(value)
                }
                else {
                    let value = s.parse().unwrap();
                    ast::decimal_literal(value)
                }
            },
            _ => {
                self.lv -= 1;
                return Err(self.parseerror("expected decimal literal"))
            }
        };
        self.lv -= 1;
        Ok(expr)
    }
}
