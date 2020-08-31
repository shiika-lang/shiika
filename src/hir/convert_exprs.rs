use crate::ast::*;
use crate::error;
use crate::error::Error;
use crate::hir::hir_maker::HirMaker;
use crate::hir::hir_maker_context::*;
use crate::hir::*;
use crate::parser::token::Token;
use crate::type_checking;

impl HirMaker {
    pub(super) fn convert_exprs(
        &mut self,
        exprs: &[AstExpression],
    ) -> Result<HirExpressions, Error> {
        let hir_exprs = exprs
            .iter()
            .map(|expr| self.convert_expr(expr))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(HirExpressions::new(hir_exprs))
    }

    pub(super) fn convert_expr(&mut self, expr: &AstExpression) -> Result<HirExpression, Error> {
        match &expr.body {
            AstExpressionBody::LogicalNot { expr } => self.convert_logical_not(expr),
            AstExpressionBody::LogicalAnd { left, right } => self.convert_logical_and(left, right),
            AstExpressionBody::LogicalOr { left, right } => self.convert_logical_or(left, right),
            AstExpressionBody::If {
                cond_expr,
                then_exprs,
                else_exprs,
            } => self.convert_if_expr(cond_expr, then_exprs, else_exprs),

            AstExpressionBody::While {
                cond_expr,
                body_exprs,
            } => self.convert_while_expr(cond_expr, body_exprs),

            AstExpressionBody::Break => self.convert_break_expr(),

            AstExpressionBody::LVarAssign { name, rhs, is_var } => {
                self.convert_lvar_assign(name, &*rhs, is_var)
            }

            AstExpressionBody::IVarAssign { name, rhs, is_var } => {
                self.convert_ivar_assign(name, &*rhs, is_var)
            }

            AstExpressionBody::ConstAssign { names, rhs } => {
                self.convert_const_assign(names, &*rhs)
            }

            AstExpressionBody::MethodCall {
                receiver_expr,
                method_name,
                arg_exprs,
                ..
            } => self.convert_method_call(receiver_expr, method_name, arg_exprs),

            AstExpressionBody::LambdaExpr { params, exprs } => {
                self.convert_lambda_expr(params, exprs)
            }

            AstExpressionBody::BareName(name) => self.convert_bare_name(name),

            AstExpressionBody::IVarRef(names) => self.convert_ivar_ref(names),

            AstExpressionBody::ConstRef(names) => self.convert_const_ref(names),

            AstExpressionBody::PseudoVariable(token) => self.convert_pseudo_variable(token),

            AstExpressionBody::ArrayLiteral(exprs) => self.convert_array_literal(exprs),

            AstExpressionBody::FloatLiteral { value } => Ok(Hir::float_literal(*value)),

            AstExpressionBody::DecimalLiteral { value } => Ok(Hir::decimal_literal(*value)),

            AstExpressionBody::StringLiteral { content } => self.convert_string_literal(content),
            //x => panic!("TODO: {:?}", x)
        }
    }

    fn convert_logical_not(&mut self, expr: &AstExpression) -> Result<HirExpression, Error> {
        let expr_hir = self.convert_expr(expr)?;
        type_checking::check_logical_operator_ty(&expr_hir.ty, "argument of logical not")?;
        Ok(Hir::logical_not(expr_hir))
    }

    fn convert_logical_and(
        &mut self,
        left: &AstExpression,
        right: &AstExpression,
    ) -> Result<HirExpression, Error> {
        let left_hir = self.convert_expr(left)?;
        let right_hir = self.convert_expr(right)?;
        type_checking::check_logical_operator_ty(&left_hir.ty, "lhs of logical and")?;
        type_checking::check_logical_operator_ty(&right_hir.ty, "rhs of logical and")?;
        Ok(Hir::logical_and(left_hir, right_hir))
    }

    fn convert_logical_or(
        &mut self,
        left: &AstExpression,
        right: &AstExpression,
    ) -> Result<HirExpression, Error> {
        let left_hir = self.convert_expr(left)?;
        let right_hir = self.convert_expr(right)?;
        type_checking::check_logical_operator_ty(&left_hir.ty, "lhs of logical or")?;
        type_checking::check_logical_operator_ty(&right_hir.ty, "rhs of logical or")?;
        Ok(Hir::logical_or(left_hir, right_hir))
    }

    fn convert_if_expr(
        &mut self,
        cond_expr: &AstExpression,
        then_exprs: &[AstExpression],
        else_exprs: &Option<Vec<AstExpression>>,
    ) -> Result<HirExpression, Error> {
        let cond_hir = self.convert_expr(cond_expr)?;
        type_checking::check_condition_ty(&cond_hir.ty, "if")?;

        let then_hirs = self.convert_exprs(then_exprs)?;
        let else_hirs = match else_exprs {
            Some(exprs) => Some(self.convert_exprs(exprs)?),
            None => None,
        };
        // TODO: then and else must have conpatible type
        Ok(Hir::if_expression(
            then_hirs.ty.clone(),
            cond_hir,
            then_hirs,
            else_hirs,
        ))
    }

    fn convert_while_expr(
        &mut self,
        cond_expr: &AstExpression,
        body_exprs: &[AstExpression],
    ) -> Result<HirExpression, Error> {
        let cond_hir = self.convert_expr(cond_expr)?;
        type_checking::check_condition_ty(&cond_hir.ty, "while")?;

        let body_hirs = self.convert_exprs(body_exprs)?;
        Ok(Hir::while_expression(cond_hir, body_hirs))
    }

    fn convert_break_expr(&mut self) -> Result<HirExpression, Error> {
        Ok(Hir::break_expression())
    }

    fn convert_lvar_assign(
        &mut self,
        name: &str,
        rhs: &AstExpression,
        is_var: &bool,
    ) -> Result<HirExpression, Error> {
        let expr = self.convert_expr(rhs)?;
        let ctx = self.ctx_mut();
        match ctx.lvars.get(name) {
            Some(lvar) => {
                // Reassigning
                if lvar.readonly {
                    return Err(error::program_error(&format!(
                        "cannot reassign to {} (Hint: declare it with `var')",
                        name
                    )));
                } else if *is_var {
                    return Err(error::program_error(&format!(
                        "variable `{}' already exists",
                        name
                    )));
                } else {
                    type_checking::check_reassign_var(&lvar.ty, &expr.ty, name)?;
                }
            }
            None => {
                // Newly introduced lvar
                ctx.lvars.insert(
                    name.to_string(),
                    CtxLVar {
                        name: name.to_string(),
                        ty: expr.ty.clone(),
                        readonly: !is_var,
                    },
                );
            }
        }

        Ok(Hir::assign_lvar(name, expr))
    }

    fn convert_ivar_assign(
        &mut self,
        name: &str,
        rhs: &AstExpression,
        is_var: &bool,
    ) -> Result<HirExpression, Error> {
        let expr = self.convert_expr(rhs)?;
        let ctx = self.method_ctx().ok_or_else(|| {
            error::program_error(&format!("cannot assign ivar `{}' out of a method", name))
        })?;

        if ctx.is_initializer {
            let idx = self.declare_ivar(name, &expr.ty, !is_var)?;
            return Ok(Hir::assign_ivar(name, idx, expr, *is_var));
        }

        if let Some(ivar) = self.class_dict.find_ivar(&ctx.self_ty.fullname, name) {
            if ivar.readonly {
                return Err(error::program_error(&format!(
                    "instance variable `{}' is readonly",
                    name
                )));
            }
            if !ivar.ty.equals_to(&expr.ty) {
                // TODO: Subtype (@obj = 1, etc.)
                return Err(error::type_error(&format!(
                    "instance variable `{}' has type {:?} but tried to assign a {:?}",
                    name, ivar.ty, expr.ty
                )));
            }
            Ok(Hir::assign_ivar(name, ivar.idx, expr, false))
        } else {
            Err(error::program_error(&format!(
                "instance variable `{}' not found",
                name
            )))
        }
    }

    /// Declare a new ivar
    fn declare_ivar(&mut self, name: &str, ty: &TermTy, readonly: bool) -> Result<usize, Error> {
        let ctx = self.method_ctx_mut().unwrap();
        if let Some(super_ivar) = ctx.super_ivars.get(name) {
            if super_ivar.ty != *ty {
                return Err(error::type_error(&format!(
                    "type of {} of {:?} is {:?} but it is defined as {:?} in the superclass",
                    &name, &ctx.self_ty, ty, super_ivar.ty
                )));
            }
            if super_ivar.readonly != readonly {
                return Err(error::type_error(&format!(
                    "mutability of {} of {:?} differs from the inherited one",
                    &name, &ctx.self_ty
                )));
            }
            // This is not a declaration (assigning to an ivar defined in superclass)
            return Ok(super_ivar.idx);
        }
        // TODO: check duplicates
        let idx = ctx.super_ivars.len() + ctx.iivars.len();
        ctx.iivars.insert(
            name.to_string(),
            SkIVar {
                idx,
                name: name.to_string(),
                ty: ty.clone(),
                readonly,
            },
        );
        Ok(idx)
    }

    fn convert_const_assign(
        &mut self,
        names: &[String],
        rhs: &AstExpression,
    ) -> Result<HirExpression, Error> {
        let name = const_firstname(&names.join("::")); // TODO: pass entire `names` rather than ConstFirstname?
        let fullname = self.register_const(&name, &rhs)?;
        Ok(Hir::assign_const(fullname, self.convert_expr(rhs)?))
    }

    fn convert_method_call(
        &mut self,
        receiver_expr: &Option<Box<AstExpression>>,
        method_name: &MethodFirstname,
        arg_exprs: &[AstExpression],
    ) -> Result<HirExpression, Error> {
        let receiver_hir = match receiver_expr {
            Some(expr) => self.convert_expr(&expr)?,
            // Implicit self
            _ => self.convert_self_expr()?,
        };
        // TODO: arg types must match with method signature
        let arg_hirs = arg_exprs
            .iter()
            .map(|arg_expr| self.convert_expr(arg_expr))
            .collect::<Result<Vec<_>, _>>()?;

        self.make_method_call(receiver_hir, &method_name, arg_hirs)
    }

    fn make_method_call(
        &self,
        receiver_hir: HirExpression,
        method_name: &MethodFirstname,
        arg_hirs: Vec<HirExpression>,
    ) -> Result<HirExpression, Error> {
        let specialized = receiver_hir.ty.is_specialized();
        let class_fullname = &receiver_hir.ty.fullname;
        let (sig, found_class_name) = self
            .class_dict
            .lookup_method(&receiver_hir.ty, method_name)?;

        let param_tys = arg_hirs.iter().map(|expr| &expr.ty).collect::<Vec<_>>();
        type_checking::check_method_args(&sig, &param_tys, &receiver_hir, &arg_hirs)?;

        let receiver = if &found_class_name != class_fullname {
            // Upcast needed
            Hir::bit_cast(found_class_name.instance_ty(), receiver_hir)
        } else {
            receiver_hir
        };

        let args;
        if specialized {
            args = arg_hirs
                .into_iter()
                .map(|expr| Hir::bit_cast(ty::raw("Object"), expr))
                .collect::<Vec<_>>();
        } else {
            args = arg_hirs;
        }

        let mut ret = Hir::method_call(sig.ret_ty.clone(), receiver, sig.fullname.clone(), args);
        if specialized {
            ret = Hir::bit_cast(sig.ret_ty, ret)
        }
        Ok(ret)
    }

    fn convert_lambda_expr(
        &mut self,
        params: &[ast::Param],
        exprs: &[AstExpression],
    ) -> Result<HirExpression, Error> {
        self.lambda_ct += 1;
        let lambda_id = self.lambda_ct;
        let hir_params = signature::convert_params(params, &[]);
        self.push_ctx(HirMakerContext::lambda_ctx(self.ctx(), hir_params.clone()));
        let hir_exprs = self.convert_exprs(exprs)?;
        // This pops ctx
        let capture_exprs = self.resolve_lambda_captures();
        Ok(Hir::lambda_expr(
            lambda_id,
            hir_params,
            hir_exprs,
            capture_exprs,
        ))
    }

    /// Resolve LambdaCapture into HirExpression
    /// Also, concat lambda_captures to outer_captures
    fn resolve_lambda_captures(&mut self) -> Vec<HirExpression> {
        let lambda_ctx = self.pop_ctx();
        let ctx = self.ctx_mut();
        lambda_ctx
            .captures
            .into_iter()
            .map(|cap| {
                if cap.ctx_depth == ctx.depth {
                    match cap.detail {
                        LambdaCaptureDetail::CapLVar { name } => Hir::lvar_ref(cap.ty, name),
                        LambdaCaptureDetail::CapFnArg { idx } => Hir::hir_arg_ref(cap.ty, idx),
                    }
                } else {
                    let ty = cap.ty.clone();
                    ctx.captures.push(cap);
                    let cidx = ctx.captures.len() - 1;
                    Hir::lambda_capture_ref(ty, cidx)
                }
            })
            .collect()
    }

    /// Generate local variable reference or method call with implicit receiver(self)
    fn convert_bare_name(&mut self, name: &str) -> Result<HirExpression, Error> {
        if let Some(expr) = self.lookup_var(name) {
            Ok(expr)
        } else {
            Err(error::program_error(&format!(
                "variable `{}' was not found",
                name
            )))
        }
    }

    /// Lookup variable of the name.
    /// If it is a free variable, ctx.captures will be modified
    fn lookup_var(&mut self, name: &str) -> Option<HirExpression> {
        let ctx = self.ctx();
        if let Some(lvar) = ctx.find_lvar(name) {
            return Some(Hir::lvar_ref(lvar.ty.clone(), name.to_string()));
        }
        if let Some((idx, param)) = ctx.find_fn_arg(name) {
            return Some(Hir::hir_arg_ref(param.ty.clone(), idx));
        }
        if let Some(outer_ctx) = self.outer_ctx() {
            let l = ctx.captures.len();
            if let Some((cap, expr)) = self.lookup_var_in_outer_scope(l, outer_ctx, name) {
                self.ctx_mut().captures.push(cap);
                return Some(expr);
            }
        }
        None
    }

    fn lookup_var_in_outer_scope(
        &self,
        cidx: usize,
        ctx: &HirMakerContext,
        name: &str,
    ) -> Option<(LambdaCapture, HirExpression)> {
        if let Some(lvar) = ctx.find_lvar(name) {
            let cap = LambdaCapture {
                ctx_depth: ctx.depth,
                ty: lvar.ty.clone(),
                detail: LambdaCaptureDetail::CapLVar {
                    name: name.to_string(),
                },
            };
            return Some((cap, Hir::lambda_capture_ref(lvar.ty.clone(), cidx)));
        }
        if let Some((idx, param)) = ctx.find_fn_arg(name) {
            let cap = LambdaCapture {
                ctx_depth: ctx.depth,
                ty: param.ty.clone(),
                detail: LambdaCaptureDetail::CapFnArg { idx },
            };
            return Some((cap, Hir::lambda_capture_ref(param.ty.clone(), cidx)));
        }

        // TODO: It may be a nullary method call

        // Lookup in the next surrounding context
        self.outer_lvar_scope_of(ctx)
            .map(|outer_ctx| self.lookup_var_in_outer_scope(cidx, &*outer_ctx, name))
            .flatten()
    }

    fn convert_ivar_ref(&self, name: &str) -> Result<HirExpression, Error> {
        let method_ctx = self.method_ctx().ok_or_else(|| {
            error::program_error(&format!("referring ivar `{}' out of a method", name))
        })?;
        match self
            .class_dict
            .find_ivar(&method_ctx.self_ty.fullname, name)
        {
            Some(ivar) => Ok(Hir::ivar_ref(ivar.ty.clone(), name.to_string(), ivar.idx)),
            None => Err(error::program_error(&format!(
                "ivar `{}' was not found",
                name
            ))),
        }
    }

    /// Resolve constant name
    fn convert_const_ref(&self, names: &[String]) -> Result<HirExpression, Error> {
        // TODO: Resolve using ctx
        let fullname = ConstFullname("::".to_string() + &names.join("::"));
        match self.constants.get(&fullname) {
            Some(ty) => Ok(Hir::const_ref(ty.clone(), fullname)),
            None => {
                let c = class_fullname(&names.join("::"));
                if self.class_dict.class_exists(&c.0) {
                    Ok(Hir::const_ref(c.class_ty(), fullname))
                } else {
                    Err(error::program_error(&format!(
                        "constant `{:?}' was not found",
                        fullname
                    )))
                }
            }
        }
    }

    fn convert_pseudo_variable(&self, token: &Token) -> Result<HirExpression, Error> {
        match token {
            Token::KwSelf => self.convert_self_expr(),
            Token::KwTrue => Ok(Hir::boolean_literal(true)),
            Token::KwFalse => Ok(Hir::boolean_literal(false)),
            _ => panic!("[BUG] not a pseudo variable token: {:?}", token),
        }
    }

    /// Generate HIR for an array literal
    /// `[x,y]` is expanded into `tmp = Array<Object>.new; tmp.push(x); tmp.push(y)`
    fn convert_array_literal(
        &mut self,
        item_exprs: &[AstExpression],
    ) -> Result<HirExpression, Error> {
        let item_exprs = item_exprs
            .iter()
            .map(|expr| self.convert_expr(expr))
            .collect::<Result<Vec<_>, _>>()?;

        // TODO #102: Support empty array literal
        let mut item_ty = item_exprs[0].ty.clone();
        for expr in &item_exprs {
            item_ty = self.nearest_common_ancestor_type(&item_ty, &expr.ty)
        }
        let ary_ty = ty::spe("Array", vec![item_ty]);
        let upper_bound_ty = ty::raw("Object");

        let tmp = self.gensym();
        let mut exprs = vec![];

        // `tmp = Array.new`
        exprs.push(Hir::assign_lvar(
            &tmp,
            Hir::method_call(
                ary_ty.clone(),
                Hir::const_ref(ty::meta("Array"), const_fullname("::Array")),
                method_fullname(&class_fullname("Meta:Array"), "new"),
                vec![Hir::decimal_literal(item_exprs.len() as i32)],
            ),
        ));
        // `tmp.push(item)`
        for expr in item_exprs {
            exprs.push(Hir::method_call(
                ty::raw("Void"),
                Hir::lvar_ref(ary_ty.clone(), tmp.clone()),
                method_fullname(&class_fullname("Array"), "push"),
                vec![Hir::bit_cast(upper_bound_ty.clone(), expr)],
            ))
        }
        exprs.push(Hir::lvar_ref(ary_ty.clone(), tmp));

        Ok(Hir::array_literal(exprs, ary_ty))
    }

    fn convert_self_expr(&self) -> Result<HirExpression, Error> {
        let ctx = self.ctx();
        Ok(Hir::self_expression(ctx.self_ty.clone()))
    }

    fn convert_string_literal(&mut self, content: &str) -> Result<HirExpression, Error> {
        let idx = self.register_string_literal(content);
        Ok(Hir::string_literal(idx))
    }

    pub(super) fn register_string_literal(&mut self, content: &str) -> usize {
        let idx = self.str_literals.len();
        self.str_literals.push(content.to_string());
        idx
    }

    /// Return the nearest common ancestor of the classes
    fn nearest_common_ancestor_type(&self, ty1: &TermTy, ty2: &TermTy) -> TermTy {
        let ancestors1 = self.class_dict.ancestor_types(ty1);
        let ancestors2 = self.class_dict.ancestor_types(ty2);
        for t2 in ancestors2 {
            if let Some(eq) = ancestors1.iter().find(|t1| t1.equals_to(&t2)) {
                return eq.clone();
            }
        }
        panic!("[BUG] nearest_common_ancestor_type not found");
    }
}
