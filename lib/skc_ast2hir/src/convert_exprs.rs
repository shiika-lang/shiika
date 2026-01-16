pub mod block;
mod lvar;
mod method_call;
pub mod params;
use crate::error;
use crate::hir_maker::extract_lvars;
use crate::hir_maker::HirMaker;
use crate::hir_maker_context::*;
use crate::pattern_match;
use crate::type_system::type_checking;
use anyhow::Result;
use lvar::{LVarDetail, LVarInfo};
use shiika_ast::Token;
use shiika_ast::*;
use shiika_core::{names::*, ty, ty::*};
use skc_hir::*;

impl<'hir_maker> HirMaker<'hir_maker> {
    pub(super) fn convert_exprs(&mut self, exprs: &[AstExpression]) -> Result<HirExpression> {
        let hir_exprs = exprs
            .iter()
            .map(|expr| self.convert_expr(expr))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Hir::expressions(hir_exprs))
    }

    pub(super) fn convert_expr(&mut self, expr: &AstExpression) -> Result<HirExpression> {
        // Debug helper: print the expr under processing
        //println!(
        //    "{}",
        //    skc_error::build_report("-".to_string(), &expr.locs, |r, locs_span| {
        //        r.with_label(skc_error::Label::new(locs_span).with_message(""))
        //    })
        //);
        match &expr.body {
            AstExpressionBody::LogicalNot { expr: arg_expr } => {
                self.convert_logical_not(arg_expr, &expr.locs)
            }
            AstExpressionBody::LogicalAnd { left, right } => {
                self.convert_logical_and(left, right, &expr.locs)
            }
            AstExpressionBody::LogicalOr { left, right } => {
                self.convert_logical_or(left, right, &expr.locs)
            }
            AstExpressionBody::If {
                cond_expr,
                then_exprs,
                else_exprs,
            } => self.convert_if_expr(cond_expr, then_exprs, else_exprs, &expr.locs),

            AstExpressionBody::Match { cond_expr, clauses } => {
                self.convert_match_expr(cond_expr, clauses, &expr.locs)
            }

            AstExpressionBody::While {
                cond_expr,
                body_exprs,
            } => self.convert_while_expr(cond_expr, body_exprs, &expr.locs),

            AstExpressionBody::Break => self.convert_break_expr(&expr.locs),

            AstExpressionBody::Return { arg } => self.convert_return_expr(arg, &expr.locs),

            AstExpressionBody::LVarDecl {
                name,
                rhs,
                readonly,
            } => self.convert_lvar_decl(name, rhs, readonly, &expr.locs),

            AstExpressionBody::LVarAssign { name, rhs } => {
                self.convert_lvar_assign(name, rhs, &expr.locs)
            }

            AstExpressionBody::IVarDecl {
                name,
                rhs,
                readonly,
            } => self.convert_ivar_decl(name, rhs, readonly, &expr.locs),

            AstExpressionBody::IVarAssign { name, rhs } => {
                self.convert_ivar_assign(name, rhs, &expr.locs)
            }

            AstExpressionBody::ConstAssign { names, rhs } => {
                self.convert_const_assign(names, rhs, &expr.locs)
            }

            AstExpressionBody::MethodCall(AstMethodCall {
                receiver_expr,
                method_name,
                args,
                type_args,
                ..
            }) => method_call::convert_method_call(
                self,
                receiver_expr,
                method_name,
                args,
                type_args,
                &expr.locs,
            ),

            AstExpressionBody::LambdaInvocation { fn_expr, args } => {
                let hir_fn_expr = self.convert_expr(fn_expr)?;
                method_call::convert_lambda_invocation(self, hir_fn_expr, args, &expr.locs)
            }

            AstExpressionBody::LambdaExpr {
                params,
                exprs,
                is_fn,
            } => self.convert_lambda_expr(params, exprs, is_fn, &expr.locs),

            // Note: there is no `AstExpressionBody::LVarRef` because it is included in this.
            AstExpressionBody::BareName(name) => self.convert_bare_name(name, &expr.locs),

            AstExpressionBody::IVarRef(name) => self.convert_ivar_ref(name, &expr.locs),

            AstExpressionBody::CapitalizedName(names) => {
                self.convert_capitalized_name(names, &expr.locs)
            }

            AstExpressionBody::SpecializeExpression { base_name, args } => {
                self.convert_specialize_expr(base_name, args, &expr.locs)
            }

            AstExpressionBody::PseudoVariable(token) => {
                self.convert_pseudo_variable(token, &expr.locs)
            }

            AstExpressionBody::ArrayLiteral(exprs) => self.convert_array_literal(exprs, &expr.locs),

            AstExpressionBody::FloatLiteral { value } => {
                Ok(Hir::float_literal(*value, expr.locs.clone()))
            }

            AstExpressionBody::DecimalLiteral { value } => {
                Ok(Hir::decimal_literal(*value, expr.locs.clone()))
            }

            AstExpressionBody::StringLiteral { content } => {
                Ok(self.convert_string_literal(content, &expr.locs))
            } //x => panic!("TODO: {:?}", x)
        }
    }

    fn convert_logical_not(
        &mut self,
        expr: &AstExpression,
        locs: &LocationSpan,
    ) -> Result<HirExpression> {
        let expr_hir = self.convert_expr(expr)?;
        type_checking::check_logical_operator_ty(&expr_hir.ty, "argument of logical not")?;
        Ok(Hir::logical_not(expr_hir, locs.clone()))
    }

    fn convert_logical_and(
        &mut self,
        left: &AstExpression,
        right: &AstExpression,
        locs: &LocationSpan,
    ) -> Result<HirExpression> {
        let left_hir = self.convert_expr(left)?;
        let right_hir = self.convert_expr(right)?;
        type_checking::check_logical_operator_ty(&left_hir.ty, "lhs of logical and")?;
        type_checking::check_logical_operator_ty(&right_hir.ty, "rhs of logical and")?;
        Ok(Hir::logical_and(left_hir, right_hir, locs.clone()))
    }

    fn convert_logical_or(
        &mut self,
        left: &AstExpression,
        right: &AstExpression,
        locs: &LocationSpan,
    ) -> Result<HirExpression> {
        let left_hir = self.convert_expr(left)?;
        let right_hir = self.convert_expr(right)?;
        type_checking::check_logical_operator_ty(&left_hir.ty, "lhs of logical or")?;
        type_checking::check_logical_operator_ty(&right_hir.ty, "rhs of logical or")?;
        Ok(Hir::logical_or(left_hir, right_hir, locs.clone()))
    }

    fn convert_if_expr(
        &mut self,
        cond_expr: &AstExpression,
        then_exprs: &[AstExpression],
        else_exprs: &Option<Vec<AstExpression>>,
        locs: &LocationSpan,
    ) -> Result<HirExpression> {
        let cond_hir = self.convert_expr(cond_expr)?;
        type_checking::check_condition_ty(&cond_hir.ty, "if")?;

        let mut if_ctxs = vec![];

        self.ctx_stack.push(HirMakerContext::if_ctx());
        let mut then_hirs = self.convert_exprs(then_exprs)?;
        if_ctxs.push(self.ctx_stack.pop_if_ctx());
        self.ctx_stack.push(HirMakerContext::if_ctx());

        let mut else_hirs = match else_exprs {
            Some(exprs) => self.convert_exprs(exprs)?,
            None => Hir::expressions(vec![]),
        };
        let else_ctx = self.ctx_stack.pop_if_ctx();
        if_ctxs.push(else_ctx);

        let if_ty = if then_hirs.ty.is_never_type() {
            else_hirs.ty.clone()
        } else if else_hirs.ty.is_never_type() {
            then_hirs.ty.clone()
        } else if then_hirs.ty.is_void_type() {
            else_hirs = else_hirs.voidify();
            ty::raw("Void")
        } else if else_hirs.ty.is_void_type() {
            then_hirs = then_hirs.voidify();
            ty::raw("Void")
        } else {
            let ty = type_checking::check_if_body_ty(
                &self.class_dict,
                &then_hirs.ty,
                then_hirs.locs.clone(),
                &else_hirs.ty,
                else_hirs.locs.clone(),
            )?;
            if !then_hirs.ty.equals_to(&ty) {
                then_hirs = Hir::bit_cast(ty.clone(), then_hirs);
            }
            if !else_hirs.ty.equals_to(&ty) {
                else_hirs = Hir::bit_cast(ty.clone(), else_hirs);
            }
            ty
        };

        let lvars = if_ctxs
            .iter()
            .flat_map(|if_ctx| {
                if_ctx.lvars.iter().fold(vec![], |mut init, (key, value)| {
                    let hirlvar = HirLVar {
                        name: key.clone(),
                        ty: value.ty.clone(),
                        captured: value.captured,
                    };
                    init.push(hirlvar);
                    init
                })
            })
            .collect::<Vec<_>>();

        Ok(Hir::if_expression(
            if_ty,
            cond_hir,
            then_hirs,
            else_hirs,
            lvars,
            locs.clone(),
        ))
    }

    fn convert_match_expr(
        &mut self,
        cond_expr: &AstExpression,
        clauses: &[AstMatchClause],
        _locs: &LocationSpan,
    ) -> Result<HirExpression> {
        pattern_match::convert_match_expr(self, cond_expr, clauses)
    }

    fn convert_while_expr(
        &mut self,
        cond_expr: &AstExpression,
        body_exprs: &[AstExpression],
        locs: &LocationSpan,
    ) -> Result<HirExpression> {
        let cond_hir = self.convert_expr(cond_expr)?;
        type_checking::check_condition_ty(&cond_hir.ty, "while")?;

        self.ctx_stack.push(HirMakerContext::while_ctx());
        let body_hirs = self.convert_exprs(body_exprs)?;

        let lvars = Vec::from_iter(self.ctx_stack.pop_while_ctx().lvars.iter().map(
            |(key, value)| HirLVar {
                name: key.clone(),
                ty: value.ty.clone(),
                captured: value.captured,
            },
        ));

        Ok(Hir::while_expression(
            cond_hir,
            body_hirs,
            lvars,
            locs.clone(),
        ))
    }

    fn convert_break_expr(&mut self, locs: &LocationSpan) -> Result<HirExpression> {
        let from;
        match self.ctx_stack.loop_ctx_mut() {
            Some(HirMakerContext::Lambda(lambda_ctx)) => {
                if lambda_ctx.is_fn {
                    return Err(error::program_error("`break' inside a fn"));
                } else {
                    // OK for now. This `break` still may be invalid
                    // (eg. `ary.map{ break }`) but it cannot be checked here
                    lambda_ctx.has_break = true;
                    from = HirBreakFrom::Block;
                }
            }
            Some(HirMakerContext::While(_)) => {
                from = HirBreakFrom::While;
            }
            _ => {
                return Err(error::program_error("`break' outside a loop"));
            }
        }
        Ok(Hir::break_expression(from, locs.clone()))
    }

    fn convert_return_expr(
        &mut self,
        arg: &Option<Box<AstExpression>>,
        locs: &LocationSpan,
    ) -> Result<HirExpression> {
        let from = self._validate_return()?;
        let arg_expr = if let Some(x) = arg {
            self.convert_expr(x)?
        } else {
            Hir::const_ref(
                ty::raw("Void"),
                toplevel_const("Void"),
                LocationSpan::todo(),
            )
        };
        let merge_ty = self._validate_return_type(&arg_expr.ty, locs)?;
        let cast = Hir::bit_cast(merge_ty, arg_expr);
        Ok(Hir::return_expression(from, cast, locs.clone()))
    }

    /// Check if `return' is valid in the current context
    fn _validate_return(&self) -> Result<HirReturnFrom> {
        if let Some(lambda_ctx) = self.ctx_stack.lambda_ctx() {
            if lambda_ctx.is_fn {
                Ok(HirReturnFrom::Fn)
            } else if self.ctx_stack.method_ctx().is_some() {
                Err(error::program_error(
                    "`return' in a block is not supported (#266)",
                ))
                //Ok(HirReturnFrom::Block)
            } else {
                Err(error::program_error("`return' outside a loop"))
            }
        } else if self.ctx_stack.method_ctx().is_some() {
            Ok(HirReturnFrom::Method)
        } else {
            Err(error::program_error("`return' outside a loop"))
        }
    }

    /// Check if the argument of `return' is valid
    pub fn _validate_return_type(&self, arg_ty: &TermTy, locs: &LocationSpan) -> Result<TermTy> {
        if self.ctx_stack.lambda_ctx().is_some() {
            // TODO: check arg_ty matches to fn's return type
            Ok(arg_ty.clone())
        } else if let Some(method_ctx) = &self.ctx_stack.method_ctx() {
            type_checking::check_return_arg_type(
                &self.class_dict,
                arg_ty,
                &method_ctx.signature,
                locs,
            )?;
            Ok(method_ctx.signature.ret_ty.clone())
        } else {
            Err(error::program_error("`return' outside a method"))
        }
    }

    /// Local variable declaration
    /// `let a = ...` or `var a = ...`
    fn convert_lvar_decl(
        &mut self,
        name: &str,
        rhs: &AstExpression,
        readonly: &bool,
        locs: &LocationSpan,
    ) -> Result<HirExpression> {
        if self._lookup_var(name, locs.clone())?.is_some() {
            return Err(error::lvar_redeclaration(name, locs));
        }
        let expr = self.convert_expr(rhs)?;
        self.ctx_stack
            .declare_lvar(name, expr.ty.clone(), *readonly);
        Ok(Hir::lvar_assign(name.to_string(), expr, locs.clone()))
    }

    /// Local variable reassignment (`a = ...`)
    fn convert_lvar_assign(
        &mut self,
        name: &str,
        rhs: &AstExpression,
        locs: &LocationSpan,
    ) -> Result<HirExpression> {
        let expr = self.convert_expr(rhs)?;
        if let (Some(mut lvar_info), opt_cidx) = self._find_var(name, locs.clone(), true)? {
            if lvar_info.ty != expr.ty {
                if let Some(t) = self
                    .class_dict
                    .nearest_common_ancestor(&lvar_info.ty, &expr.ty)
                {
                    // Upgrade lvar type (eg. from `None` to `Maybe<Int>`)
                    lvar_info.ty = t.clone();
                    if let Some(cidx) = opt_cidx {
                        self.ctx_stack
                            .lambda_ctx_mut()
                            .unwrap()
                            .update_capture_ty(cidx, t);
                    }
                } else {
                    return Err(type_checking::invalid_reassign_error(
                        &lvar_info.ty,
                        &expr.ty,
                        name,
                    ));
                }
            }
            Ok(lvar_info.assign_expr(expr))
        } else {
            Err(error::assign_to_undeclared_lvar(name, locs))
        }
    }

    /// Instance variable declaration
    fn convert_ivar_decl(
        &mut self,
        name: &str,
        rhs: &AstExpression,
        readonly: &bool,
        locs: &LocationSpan,
    ) -> Result<HirExpression> {
        if !self.ctx_stack.in_initializer() {
            return Err(error::ivar_decl_outside_initializer(name, locs));
        }
        let expr = self.convert_expr(rhs)?;
        let base_ty = self.ctx_stack.self_ty().erasure_ty();
        let idx = self.declare_ivar(name, &expr.ty, *readonly)?;
        Ok(Hir::ivar_assign(
            name,
            idx,
            expr,
            !*readonly,
            base_ty,
            locs.clone(),
        ))
    }

    /// Instance variable reassignment (`@a = ...`)
    fn convert_ivar_assign(
        &mut self,
        name: &str,
        rhs: &AstExpression,
        locs: &LocationSpan,
    ) -> Result<HirExpression> {
        let expr = self.convert_expr(rhs)?;
        let base_ty = self.ctx_stack.self_ty().erasure_ty();

        if let Some(ivar) = self
            .class_dict
            .find_ivar(&base_ty.fullname.to_class_fullname(), name)
        {
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
            Ok(Hir::ivar_assign(
                name,
                ivar.idx,
                expr,
                false,
                base_ty,
                locs.clone(),
            ))
        } else {
            Err(error::assign_to_undeclared_ivar(name, locs))
        }
    }

    /// Declare a new ivar
    fn declare_ivar(&mut self, name: &str, ty: &TermTy, readonly: bool) -> Result<usize> {
        let self_ty = &self.ctx_stack.self_ty();
        let method_ctx = self.ctx_stack.method_ctx_mut().unwrap();
        if let Some(super_ivar) = method_ctx.super_ivars.get(name) {
            if super_ivar.ty != *ty {
                return Err(error::type_error(&format!(
                    "type of {} of {:?} is {:?} but it is defined as {:?} in the superclass",
                    &name, &self_ty, ty, super_ivar.ty
                )));
            }
            if super_ivar.readonly != readonly {
                return Err(error::type_error(&format!(
                    "mutability of {} of {:?} differs from the inherited one",
                    &name, &self_ty
                )));
            }
            // This is not a declaration (assigning to an ivar defined in superclass)
            Ok(super_ivar.idx)
        } else {
            // TODO: check duplicates
            let idx = method_ctx.super_ivars.len() + method_ctx.iivars.len();
            method_ctx.iivars.insert(
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
    }

    /// Constant assignment (only occurs in the toplevel)
    fn convert_const_assign(
        &mut self,
        names: &[String],
        rhs: &AstExpression,
        locs: &LocationSpan,
    ) -> Result<HirExpression> {
        // TODO: forbid `A::B = 1`
        let fullname = toplevel_const(&names.join("::"));
        let hir_expr = self.convert_expr(rhs)?;
        self.constants.insert(fullname.clone(), hir_expr.ty.clone());
        Ok(Hir::const_assign(fullname, hir_expr, locs.clone()))
    }

    pub(super) fn convert_lambda_expr(
        &mut self,
        params: &[shiika_ast::BlockParam],
        exprs: &[AstExpression],
        is_fn: &bool,
        locs: &LocationSpan,
    ) -> Result<HirExpression> {
        let namespace = self.ctx_stack.const_scopes().next().unwrap();
        let hir_params = params::convert_block_params(
            &self.class_dict,
            &namespace,
            params,
            &self.ctx_stack.current_class_typarams(),
            &self.ctx_stack.current_method_typarams(),
            Default::default(),
        )?;

        // Convert lambda body
        self.ctx_stack
            .push(HirMakerContext::lambda(*is_fn, hir_params.clone()));
        let hir_exprs = self.convert_exprs(exprs)?;
        let mut lambda_ctx = self.ctx_stack.pop_lambda_ctx();
        let captures = self._resolve_lambda_captures(lambda_ctx.captures);
        Ok(Hir::lambda_expr(
            block::lambda_ty(&hir_params, &hir_exprs.ty),
            self.create_lambda_name(),
            hir_params,
            hir_exprs,
            captures,
            extract_lvars(&mut lambda_ctx.lvars),
            lambda_ctx.has_break,
            locs.clone(),
        ))
    }

    /// Returns a newly created name for a lambda
    pub fn create_lambda_name(&mut self) -> String {
        self.lambda_ct += 1;
        let lambda_id = self.lambda_ct;
        format!(
            "lambda_{}_in_{}",
            lambda_id,
            self.ctx_stack.describe_current_place()
        )
    }

    /// Resolve LambdaCapture into HirExpression
    /// Also handles forwarding captured variables among nested lambdas
    fn _resolve_lambda_captures(
        &mut self,
        lambda_captures: Vec<LambdaCapture>,
    ) -> Vec<HirLambdaCapture> {
        let mut ret = vec![];
        for cap in lambda_captures {
            let captured_here = if let HirMakerContext::Lambda(_) = self.ctx_stack.top() {
                cap.is_lambda_scope && cap.ctx_idx == self.ctx_stack.len() - 1
            } else {
                !cap.is_lambda_scope
            };
            if captured_here {
                // The variable is in this scope
                let detail = match cap.detail {
                    LambdaCaptureDetail::CapLVar { name }
                    | LambdaCaptureDetail::CapOmittableArg { name } => {
                        HirLambdaCaptureDetail::CaptureLVar { name }
                    }
                    LambdaCaptureDetail::CapFnArg { idx } => {
                        HirLambdaCaptureDetail::CaptureArg { idx }
                    }
                    LambdaCaptureDetail::CapMethodTyArg { idx, n_params } => {
                        HirLambdaCaptureDetail::CaptureMethodTyArg { idx, n_params }
                    }
                };
                ret.push(HirLambdaCapture {
                    ty: cap.ty,
                    upcast_needed: cap.upcast_needed,
                    readonly: cap.readonly,
                    detail,
                });
            } else {
                // The variable is in outer scope
                let ty = cap.ty.clone();
                let upcast_needed = cap.upcast_needed;
                let readonly = cap.readonly;
                let cidx = self
                    .ctx_stack
                    .lambda_ctx_mut()
                    .expect("[BUG] no lambda_ctx found")
                    .push_lambda_capture(cap);
                ret.push(HirLambdaCapture {
                    ty,
                    upcast_needed,
                    readonly,
                    detail: HirLambdaCaptureDetail::CaptureFwd { cidx },
                });
            }
        }
        ret
    }

    /// Generate local variable reference or method call with implicit receiver(self)
    fn convert_bare_name(&mut self, name: &str, locs: &LocationSpan) -> Result<HirExpression> {
        // Found a local variable
        if let (Some(lvar_info), _) = self._find_var(name, locs.clone(), false)? {
            return Ok(lvar_info.ref_expr());
        }

        method_call::convert_method_call(
            self,
            &None,
            &method_firstname(name),
            &Default::default(),
            Default::default(),
            locs,
        )
    }

    /// Return the variable of the given name, if any
    fn _lookup_var(&mut self, name: &str, locs: LocationSpan) -> Result<Option<LVarInfo>> {
        let (lvar, _) = self._find_var(name, locs, false)?;
        Ok(lvar)
    }

    /// Find the variable of the given name.
    /// If it is a free variable, lambda_ctx.captures will be modified
    fn _find_var(
        &mut self,
        name: &str,
        locs: LocationSpan,
        updating: bool,
    ) -> Result<(Option<LVarInfo>, Option<usize>)> {
        if self.ctx_stack.lambda_ctx().is_some() {
            let (mut found, opt_cap) = self.__find_var(true, name, locs, updating)?;
            let opt_cidx = if let Some(cap) = opt_cap {
                let lambda_ctx = self.ctx_stack.lambda_ctx_mut().unwrap();
                if let Some(existing) = lambda_ctx.check_already_captured(&cap) {
                    if let Some(x) = found.as_mut() {
                        x.set_cidx(existing)
                    }
                    Some(existing)
                } else {
                    let cidx = lambda_ctx.push_lambda_capture(cap);
                    if let Some(x) = found.as_mut() {
                        x.set_cidx(cidx)
                    }
                    Some(cidx)
                }
            } else {
                None
            };
            Ok((found, opt_cidx))
        } else {
            let (found, _) = self.__find_var(false, name, locs, updating)?;
            Ok((found, None))
        }
    }

    fn __find_var(
        &mut self,
        in_lambda: bool,
        name: &str,
        locs: LocationSpan,
        updating: bool,
    ) -> Result<(Option<LVarInfo>, Option<LambdaCapture>)> {
        let mut lambda_seen = false;
        let mut result = (None, None);
        let mut captured = None;
        for scope in self.ctx_stack.lvar_scopes() {
            let is_lambda_capture = in_lambda && lambda_seen;
            if let Some(lvar) = scope.lvars.get(name) {
                if updating && lvar.readonly {
                    return Err(error::program_error(&format!(
                        "cannot reassign to `{}' (Hint: declare it with `var')",
                        name
                    )));
                }
                if is_lambda_capture {
                    captured = Some((scope.ctx_idx, name));
                    let cap = LambdaCapture {
                        ctx_idx: scope.ctx_idx,
                        is_lambda_scope: scope.is_lambda_scope,
                        ty: lvar.ty.clone(),
                        upcast_needed: false,
                        readonly: lvar.readonly,
                        detail: LambdaCaptureDetail::CapLVar {
                            name: name.to_string(),
                        },
                    };
                    let lvar_info = LVarInfo {
                        ty: lvar.ty.clone(),
                        detail: LVarDetail::OuterScope_ {
                            readonly: lvar.readonly,
                        },
                        locs,
                    };
                    result = (Some(lvar_info), Some(cap));
                    break;
                } else {
                    let lvar_info = LVarInfo {
                        ty: lvar.ty.clone(),
                        detail: LVarDetail::CurrentScope {
                            name: name.to_string(),
                        },
                        locs,
                    };
                    return Ok((Some(lvar_info), None));
                }
            }
            if let Some((idx, param)) = find_param(scope.params, name) {
                if updating {
                    return Err(error::program_error(&format!(
                        "you cannot reassign to argument `{}'",
                        name
                    )));
                }
                if is_lambda_capture {
                    let detail = if param.has_default {
                        LambdaCaptureDetail::CapOmittableArg {
                            name: param.name.clone(),
                        }
                    } else {
                        LambdaCaptureDetail::CapFnArg { idx }
                    };
                    let cap = LambdaCapture {
                        ctx_idx: scope.ctx_idx,
                        is_lambda_scope: scope.is_lambda_scope,
                        ty: param.ty.clone(),
                        upcast_needed: false,
                        readonly: true,
                        detail,
                    };
                    let lvar_info = LVarInfo {
                        ty: param.ty.clone(),
                        detail: LVarDetail::OuterScope_ { readonly: true },
                        locs,
                    };
                    return Ok((Some(lvar_info), Some(cap)));
                } else {
                    let detail = if param.has_default {
                        LVarDetail::OmittableArgument {
                            name: param.name.clone(),
                        }
                    } else {
                        LVarDetail::Argument { idx }
                    };
                    let lvar_info = LVarInfo {
                        ty: param.ty.clone(),
                        detail,
                        locs,
                    };
                    return Ok((Some(lvar_info), None));
                }
            }
            if scope.is_lambda_scope {
                lambda_seen = true;
            }
        }
        if let Some((ctx_idx, name)) = captured {
            // Set `captured` to `true` so that this lvar is allocated on heap, not stack.
            // (PERF: technically, it can be on the stack if the lambda does not live
            // after returning the method.)
            let ctx = self.ctx_stack.get_mut(ctx_idx);
            ctx.set_lvar_captured(name, true);
            Ok(result)
        } else {
            Ok((None, None))
        }
    }

    fn convert_ivar_ref(&self, name: &str, locs: &LocationSpan) -> Result<HirExpression> {
        let base_ty = self.ctx_stack.self_ty().erasure_ty();
        let found = self
            .class_dict
            .find_ivar(&base_ty.fullname.to_class_fullname(), name)
            .or_else(|| {
                self.ctx_stack
                    .method_ctx()
                    .as_ref()
                    .unwrap()
                    .iivars
                    .get(name)
            });
        match found {
            Some(ivar) => Ok(Hir::ivar_ref(
                ivar.ty.clone(),
                name.to_string(),
                ivar.idx,
                base_ty,
                locs.clone(),
            )),
            None => Err(error::program_error(&format!(
                "ivar `{}' was not found",
                name
            ))),
        }
    }

    /// Resolve a capitalized identifier, which is either a constant name or
    /// a type parameter reference
    pub fn convert_capitalized_name(
        &mut self,
        name: &UnresolvedConstName,
        locs: &LocationSpan,
    ) -> Result<HirExpression> {
        // Check if it is a typaram ref
        if name.0.len() == 1 {
            let s = name.0.first().unwrap();
            if let Some(typaram_ref) = self.ctx_stack.lookup_typaram(s) {
                return Ok(self.tvar_ref(typaram_ref, locs));
            }
        }

        for namespace in self.ctx_stack.const_scopes() {
            let resolved = resolved_const_name(namespace, name.0.to_vec());
            let full = resolved.to_const_fullname();
            if let Some(ty) = self._lookup_const(&full) {
                return Ok(Hir::const_ref(ty, full, locs.clone()));
            }
        }
        Err(error::program_error(&format!(
            "constant `{:?}' was not found",
            name.0.join("::")
        )))
    }

    /// Get the value of a class-wise or method-wise type argument.
    pub fn tvar_ref(&mut self, typaram_ref: TyParamRef, locs: &LocationSpan) -> HirExpression {
        let cls_ty = typaram_ref.to_term_ty();
        match typaram_ref.kind {
            TyParamKind::Class => {
                let base_ty = self.ctx_stack.self_ty().erasure_ty();
                Hir::class_tvar_ref(cls_ty, typaram_ref, base_ty, locs.clone())
            }
            TyParamKind::Method => {
                let (method_ctx, method_ctx_idx, opt_lambda_ctx) =
                    self.ctx_stack.method_and_lambda_ctx();
                let n_params = method_ctx.signature.params.len();
                if let Some(lambda_ctx) = opt_lambda_ctx {
                    // We're in a lambda so the tyarg should be captured in it
                    let cap = LambdaCapture {
                        ctx_idx: method_ctx_idx,
                        is_lambda_scope: false,
                        ty: typaram_ref.upper_bound.to_term_ty().meta_ty(),
                        upcast_needed: false,
                        readonly: true,
                        detail: LambdaCaptureDetail::CapMethodTyArg {
                            idx: typaram_ref.idx,
                            n_params,
                        },
                    };
                    let cidx = if let Some(existing) = lambda_ctx.check_already_captured(&cap) {
                        existing
                    } else {
                        lambda_ctx.push_lambda_capture(cap)
                    };
                    Hir::lambda_capture_ref(cls_ty, cidx, true, locs.clone())
                } else {
                    // Not in a lambda so we can just get the tyarg
                    Hir::method_tvar_ref(cls_ty, typaram_ref, n_params, locs.clone())
                }
            }
        }
    }

    /// Check if a constant is registered
    fn _lookup_const(&self, full: &ConstFullname) -> Option<TermTy> {
        self.constants
            .get(full)
            .or_else(|| self.imported_constants.get(full))
            .cloned()
    }

    /// Expr of the form `A<B>`. `A` is limited to a capitalized identifier
    /// or a sequence of them (eg. `X::Y::Z`.)
    fn convert_specialize_expr(
        &mut self,
        base_name: &UnresolvedConstName,
        args: &[AstExpression],
        locs: &LocationSpan,
    ) -> Result<HirExpression> {
        debug_assert!(!args.is_empty());
        let base_expr = self.resolve_class_expr(base_name, locs)?;
        let mut arg_exprs = vec![];
        let mut type_args = vec![];
        for arg in args {
            let cls_expr = match &arg.body {
                AstExpressionBody::CapitalizedName(n) => self.resolve_class_expr(n, &arg.locs)?,
                AstExpressionBody::SpecializeExpression {
                    base_name: n,
                    args: a,
                } => self.convert_specialize_expr(n, a, locs)?,
                _ => panic!("[BUG] unexpected arg in SpecializeExpression"),
            };
            type_args.push(cls_expr.ty.as_type_argument());
            arg_exprs.push(cls_expr);
        }

        let sk_type = self
            .class_dict
            .get_type(&base_expr.ty.instance_ty().fullname);
        type_checking::check_class_specialization(sk_type, &arg_exprs, locs)?;

        let meta_spe_ty = base_expr.ty.specialized_ty(type_args);
        Ok(Hir::method_call(
            meta_spe_ty,
            base_expr,
            method_fullname_raw("Class", "<>"),
            vec![self.create_array_instance_(arg_exprs, ty::raw("Class"), LocationSpan::todo())],
            Default::default(),
            true,
        ))
    }

    pub fn resolve_class_expr(
        &mut self,
        name: &UnresolvedConstName,
        locs: &LocationSpan,
    ) -> Result<HirExpression> {
        let e = self.convert_capitalized_name(name, locs)?;
        self.assert_class_expr(&e, locs)?;
        Ok(e)
    }

    /// Check if `e` evaluates to a class object.
    fn assert_class_expr(&self, e: &HirExpression, locs: &LocationSpan) -> Result<()> {
        if e.ty.is_metaclass() || e.ty.is_typaram_ref() {
            Ok(())
        } else {
            Err(error::not_a_class_expression(&e.ty, locs))
        }
    }

    fn convert_pseudo_variable(&self, token: &Token, locs: &LocationSpan) -> Result<HirExpression> {
        match token {
            Token::KwSelf => Ok(self.convert_self_expr(locs)),
            Token::KwTrue => Ok(Hir::boolean_literal(true, locs.clone())),
            Token::KwFalse => Ok(Hir::boolean_literal(false, locs.clone())),
            _ => panic!("[BUG] not a pseudo variable token: {:?}", token),
        }
    }

    /// Generate HIR for an array literal
    fn convert_array_literal(
        &mut self,
        item_exprs: &[AstExpression],
        locs: &LocationSpan,
    ) -> Result<HirExpression> {
        let item_exprs = item_exprs
            .iter()
            .map(|expr| self.convert_expr(expr))
            .collect::<Result<Vec<_>, _>>()?;

        #[cfg(feature = "new-runtime")]
        {
            let item_ty = self.array_item_ty(&item_exprs);
            Ok(Hir::array_literal(
                ty::spe("Array", vec![item_ty]),
                item_exprs,
                locs.clone(),
            ))
        }

        #[cfg(not(feature = "new-runtime"))]
        {
            Ok(self.create_array_instance(item_exprs, locs.clone()))
        }
    }

    pub fn create_array_instance(
        &mut self,
        item_exprs: Vec<HirExpression>,
        locs: LocationSpan,
    ) -> HirExpression {
        let item_ty = self.array_item_ty(&item_exprs);
        self.create_array_instance_(item_exprs, item_ty, locs)
    }

    fn array_item_ty(&self, item_exprs: &[HirExpression]) -> TermTy {
        if item_exprs.is_empty() {
            return ty::raw("Object");
        }
        let mut item_ty = item_exprs[0].ty.clone();
        if item_exprs.len() == 1 {
            return item_ty;
        }
        for expr in item_exprs {
            item_ty = self
                .class_dict
                .nearest_common_ancestor(&item_ty, &expr.ty)
                .expect("array literal elements type mismatch");
        }
        item_ty
    }

    /// Expand `[123]` into `tmp=Array<X>.new; tmp.push(123)`
    pub fn create_array_instance_(
        &mut self,
        item_exprs: Vec<HirExpression>,
        item_ty: TermTy,
        locs: LocationSpan,
    ) -> HirExpression {
        let ary_ty = ty::spe("Array", vec![item_ty]);
        let mut exprs = vec![];

        let tmp_name = self.generate_lvar_name("ary");
        let readonly = true;
        self.ctx_stack
            .declare_lvar(&tmp_name, ary_ty.clone(), readonly);

        // `Array<X>.new`
        let call_new = Hir::method_call(
            ary_ty.clone(),
            self.get_class_object(&ary_ty.meta_ty(), &locs),
            method_fullname_raw("Meta:Array", "new"),
            vec![],
            Default::default(),
            false,
        );
        exprs.push(Hir::lvar_assign(tmp_name.clone(), call_new, locs.clone()));

        // `tmp.push(item)`
        for item_expr in item_exprs {
            exprs.push(Hir::method_call(
                ty::raw("Void"),
                Hir::lvar_ref(ary_ty.clone(), tmp_name.clone(), locs.clone()),
                method_fullname_raw("Array", "push"),
                vec![Hir::bit_cast(ty::raw("Object"), item_expr)],
                Default::default(),
                false,
            ));
        }

        exprs.push(Hir::lvar_ref(ary_ty.clone(), tmp_name, locs.clone()));
        Hir::parenthesized_expression(ary_ty, exprs, locs)
    }

    fn convert_self_expr(&self, locs: &LocationSpan) -> HirExpression {
        Hir::self_expression(self.ctx_stack.self_ty(), locs.clone())
    }

    pub(super) fn convert_string_literal(
        &mut self,
        content: &str,
        locs: &LocationSpan,
    ) -> HirExpression {
        let idx = self.register_string_literal(content);
        Hir::string_literal(idx, locs.clone())
    }

    pub(super) fn register_string_literal(&mut self, content: &str) -> usize {
        let idx = self.str_literals.len();
        self.str_literals.push(content.to_string());
        idx
    }
}
