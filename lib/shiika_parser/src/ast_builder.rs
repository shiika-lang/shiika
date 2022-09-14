use shiika_ast::{
    AstExpression, AstExpressionBody, AstMatchClause, AstMethodCall, Location, LocationSpan, Token,
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
        self.primary_expression(
            left.locs.begin.clone(),
            right.locs.end.clone(),
            AstExpressionBody::LogicalAnd {
                left: Box::new(left),
                right: Box::new(right),
            },
        )
    }

    pub fn logical_or(&self, left: AstExpression, right: AstExpression) -> AstExpression {
        self.primary_expression(
            left.locs.begin.clone(),
            right.locs.end.clone(),
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
        begin: Location,
        end: Location,
    ) -> AstExpression {
        self.non_primary_expression(
            begin,
            end,
            AstExpressionBody::LVarAssign {
                name,
                rhs: Box::new(rhs),
                is_var: true,
            },
        )
    }

    pub fn ivar_decl(
        &self,
        name: String,
        rhs: AstExpression,
        begin: Location,
        end: Location,
    ) -> AstExpression {
        self.non_primary_expression(
            begin,
            end,
            AstExpressionBody::IVarAssign {
                name,
                rhs: Box::new(rhs),
                is_var: true,
            },
        )
    }

    pub fn ivar_assign(
        &self,
        name: String,
        rhs: AstExpression,
        begin: Location,
        end: Location,
    ) -> AstExpression {
        self.non_primary_expression(
            begin,
            end,
            AstExpressionBody::IVarAssign {
                name,
                rhs: Box::new(rhs),
                is_var: false,
            },
        )
    }

    pub fn method_call(&self, primary: bool, body: AstMethodCall) -> AstExpression {
        AstExpression {
            primary,
            body: AstExpressionBody::MethodCall(body),
            locs: LocationSpan::todo(),
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
        self.non_primary_expression(
            begin,
            end,
            AstExpressionBody::MethodCall(AstMethodCall {
                receiver_expr: receiver_expr.map(Box::new),
                method_name: method_firstname(method_name),
                arg_exprs,
                type_args: Default::default(),
                has_block: false,
                may_have_paren_wo_args: false,
            }),
        )
    }

    // TODO
    // LambdaExpr {
    // BareName(String),

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
}
