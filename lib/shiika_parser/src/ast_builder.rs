use shiika_ast::{
    AstCallArgs, AstExpression, AstExpressionBody, AstMatchClause, AstMethodCall, BlockParam,
    Location, LocationSpan, Token, UnresolvedTypeName,
};
use shiika_core::names::{method_firstname, UnresolvedConstName};
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub struct AstBuilder {
    pub filepath: Rc<PathBuf>,
}

impl AstBuilder {
    pub fn new(filepath: &Rc<PathBuf>) -> AstBuilder {
        AstBuilder {
            filepath: filepath.clone(),
        }
    }

    pub fn empty() -> AstBuilder {
        AstBuilder {
            filepath: Rc::new(Path::new("").to_path_buf()),
        }
    }

    fn locs(&self, begin: Location, end: Location) -> LocationSpan {
        LocationSpan::new(&self.filepath, begin, end)
    }

    pub fn unresolved_type_name(
        &self,
        names: Vec<String>,
        args: Vec<UnresolvedTypeName>,
        begin: Location,
        end: Location,
    ) -> UnresolvedTypeName {
        UnresolvedTypeName {
            names,
            args,
            locs: self.locs(begin, end),
        }
    }

    pub fn logical_not(
        &self,
        expr: AstExpression,
        begin: Location,
        end: Location,
    ) -> AstExpression {
        self.primary_expression(
            begin,
            end,
            AstExpressionBody::LogicalNot {
                expr: Box::new(expr),
            },
        )
    }

    pub fn wrap_with_logical_not(&self, expr: AstExpression) -> AstExpression {
        let locs = expr.locs.clone();
        AstExpression {
            primary: true,
            body: AstExpressionBody::LogicalNot {
                expr: Box::new(expr),
            },
            locs,
        }
    }

    pub fn logical_and(&self, left: AstExpression, right: AstExpression) -> AstExpression {
        self.primary_expression_(
            &left.locs.clone(),
            &right.locs.clone(),
            AstExpressionBody::LogicalAnd {
                left: Box::new(left),
                right: Box::new(right),
            },
        )
    }

    pub fn logical_or(&self, left: AstExpression, right: AstExpression) -> AstExpression {
        self.primary_expression_(
            &left.locs.clone(),
            &right.locs.clone(),
            AstExpressionBody::LogicalOr {
                left: Box::new(left),
                right: Box::new(right),
            },
        )
    }

    pub fn if_expr(
        &self,
        cond_expr: AstExpression,
        then_exprs: Vec<AstExpression>,
        else_exprs: Option<Vec<AstExpression>>,
        begin: Location,
        end: Location,
    ) -> AstExpression {
        self.non_primary_expression(
            begin,
            end,
            AstExpressionBody::If {
                cond_expr: Box::new(cond_expr),
                then_exprs,
                else_exprs,
            },
        )
    }

    pub fn match_expr(
        &self,
        cond_expr: AstExpression,
        clauses: Vec<AstMatchClause>,
        begin: Location,
        end: Location,
    ) -> AstExpression {
        self.non_primary_expression(
            begin,
            end,
            AstExpressionBody::Match {
                cond_expr: Box::new(cond_expr),
                clauses,
            },
        )
    }

    pub fn while_expr(
        &self,
        cond_expr: AstExpression,
        body_exprs: Vec<AstExpression>,
        begin: Location,
        end: Location,
    ) -> AstExpression {
        self.non_primary_expression(
            begin,
            end,
            AstExpressionBody::While {
                cond_expr: Box::new(cond_expr),
                body_exprs,
            },
        )
    }

    pub fn break_expr(&self, begin: Location, end: Location) -> AstExpression {
        self.non_primary_expression(begin, end, AstExpressionBody::Break {})
    }

    pub fn return_expr(
        &self,
        arg: Option<AstExpression>,
        begin: Location,
        end: Location,
    ) -> AstExpression {
        self.non_primary_expression(
            begin,
            end,
            AstExpressionBody::Return {
                arg: arg.map(Box::new),
            },
        )
    }

    pub fn lvar_decl(
        &self,
        name: String,
        rhs: AstExpression,
        readonly: bool,
        begin: Location,
        end: Location,
    ) -> AstExpression {
        self.non_primary_expression(
            begin,
            end,
            AstExpressionBody::LVarDecl {
                name,
                rhs: Box::new(rhs),
                readonly,
            },
        )
    }

    pub fn ivar_decl(
        &self,
        name: String,
        rhs: AstExpression,
        readonly: bool,
        begin: Location,
        end: Location,
    ) -> AstExpression {
        self.non_primary_expression(
            begin,
            end,
            AstExpressionBody::IVarDecl {
                name,
                rhs: Box::new(rhs),
                readonly,
            },
        )
    }

    pub fn method_call(
        &self,
        primary: bool,
        mc: AstMethodCall,
        begin: Location,
        end: Location,
    ) -> AstExpression {
        AstExpression {
            primary,
            body: AstExpressionBody::MethodCall(mc),
            locs: LocationSpan::new(&self.filepath, begin, end),
        }
    }

    pub fn simple_method_call(
        &self,
        receiver_expr: Option<AstExpression>,
        method_name: &str,
        arg_exprs: Vec<AstExpression>,
        begin: Location,
        end: Location,
    ) -> AstExpression {
        let mut args = AstCallArgs::new();
        for e in arg_exprs {
            args.add_unnamed(e);
        }
        self.non_primary_expression(
            begin,
            end,
            AstExpressionBody::MethodCall(AstMethodCall {
                receiver_expr: receiver_expr.map(Box::new),
                method_name: method_firstname(method_name),
                args,
                type_args: Default::default(),
                may_have_paren_wo_args: false,
            }),
        )
    }

    pub fn lambda_invocation(
        &self,
        fn_expr: AstExpression,
        args: AstCallArgs,
        begin: Location,
        end: Location,
    ) -> AstExpression {
        self.primary_expression(
            begin,
            end,
            AstExpressionBody::LambdaInvocation {
                fn_expr: Box::new(fn_expr),
                args,
            },
        )
    }

    pub fn lambda_expr(
        &self,
        params: Vec<BlockParam>,
        exprs: Vec<AstExpression>,
        is_fn: bool,
        begin: Location,
        end: Location,
    ) -> AstExpression {
        self.primary_expression(
            begin,
            end,
            AstExpressionBody::LambdaExpr {
                params,
                exprs,
                is_fn,
            },
        )
    }

    pub fn bare_name(&self, name: &str, begin: Location, end: Location) -> AstExpression {
        self.primary_expression(begin, end, AstExpressionBody::BareName(name.to_string()))
    }

    pub fn ivar_ref(&self, name: String, begin: Location, end: Location) -> AstExpression {
        self.primary_expression(begin, end, AstExpressionBody::IVarRef(name))
    }

    pub fn capitalized_name(
        &self,
        name: Vec<String>,
        begin: Location,
        end: Location,
    ) -> AstExpression {
        self.primary_expression(
            begin,
            end,
            AstExpressionBody::CapitalizedName(UnresolvedConstName(name)),
        )
    }

    pub fn specialize_expr(
        &self,
        base_name: Vec<String>,
        args: Vec<AstExpression>,
        begin: Location,
        end: Location,
    ) -> AstExpression {
        self.primary_expression(
            begin,
            end,
            AstExpressionBody::SpecializeExpression {
                base_name: UnresolvedConstName(base_name),
                args,
            },
        )
    }

    pub fn pseudo_variable(&self, token: Token, begin: Location, end: Location) -> AstExpression {
        self.primary_expression(begin, end, AstExpressionBody::PseudoVariable(token))
    }

    pub fn array_literal(
        &self,
        exprs: Vec<AstExpression>,
        begin: Location,
        end: Location,
    ) -> AstExpression {
        self.primary_expression(begin, end, AstExpressionBody::ArrayLiteral(exprs))
    }

    pub fn float_literal(&self, value: f64, begin: Location, end: Location) -> AstExpression {
        self.primary_expression(begin, end, AstExpressionBody::FloatLiteral { value })
    }

    pub fn string_literal(&self, content: String, begin: Location, end: Location) -> AstExpression {
        self.primary_expression(begin, end, AstExpressionBody::StringLiteral { content })
    }

    pub fn decimal_literal(&self, value: i64, begin: Location, end: Location) -> AstExpression {
        self.primary_expression(begin, end, AstExpressionBody::DecimalLiteral { value })
    }

    fn primary_expression(
        &self,
        begin: Location,
        end: Location,
        body: AstExpressionBody,
    ) -> AstExpression {
        AstExpression {
            primary: true,
            body,
            locs: LocationSpan::new(&self.filepath, begin, end),
        }
    }

    fn primary_expression_(
        &self,
        begin: &LocationSpan,
        end: &LocationSpan,
        body: AstExpressionBody,
    ) -> AstExpression {
        AstExpression {
            primary: true,
            body,
            locs: LocationSpan::merge(begin, end),
        }
    }

    fn non_primary_expression(
        &self,
        begin: Location,
        end: Location,
        body: AstExpressionBody,
    ) -> AstExpression {
        AstExpression {
            primary: false,
            body,
            locs: LocationSpan::new(&self.filepath, begin, end),
        }
    }

    fn non_primary_expression_(
        &self,
        begin: &LocationSpan,
        end: &LocationSpan,
        body: AstExpressionBody,
    ) -> AstExpression {
        AstExpression {
            primary: false,
            body,
            locs: LocationSpan::merge(begin, end),
        }
    }

    /// Create an expression of the form `left <op> right`
    pub fn bin_op_expr(
        &self,
        left: AstExpression,
        op: &str,
        right: AstExpression,
    ) -> AstExpression {
        self.non_primary_expression_(
            &left.locs.clone(),
            &right.locs.clone(),
            AstExpressionBody::MethodCall(AstMethodCall {
                receiver_expr: Some(Box::new(left)),
                method_name: method_firstname(op),
                args: AstCallArgs::single_unnamed(right),
                type_args: vec![],
                may_have_paren_wo_args: false,
            }),
        )
    }

    /// Create an expression of the form `lhs = rhs`
    pub fn assignment(&self, lhs: AstExpression, rhs: AstExpression) -> AstExpression {
        let begin = &lhs.locs.clone();
        let end = &rhs.locs.clone();
        let body = match lhs.body {
            AstExpressionBody::BareName(s) => AstExpressionBody::LVarAssign {
                name: s,
                rhs: Box::new(rhs),
            },
            AstExpressionBody::IVarRef(name) => AstExpressionBody::IVarAssign {
                name,
                rhs: Box::new(rhs),
            },
            AstExpressionBody::CapitalizedName(names) => AstExpressionBody::ConstAssign {
                names: names.0,
                rhs: Box::new(rhs),
            },
            AstExpressionBody::MethodCall(mut x) => {
                x.args.add_unnamed(rhs);
                AstExpressionBody::MethodCall(AstMethodCall {
                    receiver_expr: x.receiver_expr,
                    method_name: x.method_name.append("="),
                    args: x.args,
                    type_args: Default::default(),
                    may_have_paren_wo_args: false,
                })
            }
            _ => panic!("[BUG] unexpectd lhs: {:?}", lhs.body),
        };
        self.non_primary_expression_(begin, end, body)
    }

    /// Extend `foo.bar` to `foo.bar args`, or
    ///        `foo`     to `foo args`.
    /// (expr must be a MethodCall or a BareName and args must not be empty)
    pub fn set_method_call_args(&self, expr: AstExpression, args: AstCallArgs) -> AstExpression {
        let begin = &expr.locs;
        let end = &args.locs().unwrap();
        match expr.body {
            AstExpressionBody::MethodCall(x) => self.non_primary_expression_(
                begin,
                end,
                AstExpressionBody::MethodCall(AstMethodCall {
                    args,
                    may_have_paren_wo_args: false,
                    ..x
                }),
            ),
            AstExpressionBody::BareName(s) => self.non_primary_expression_(
                begin,
                end,
                AstExpressionBody::MethodCall(AstMethodCall {
                    receiver_expr: None,
                    method_name: method_firstname(s),
                    args,
                    type_args: vec![],
                    may_have_paren_wo_args: false,
                }),
            ),
            b => panic!("[BUG] `extend' takes a MethodCall but got {:?}", b),
        }
    }
}
