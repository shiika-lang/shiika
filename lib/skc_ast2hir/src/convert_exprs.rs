use crate::error;
use crate::hir_maker::extract_lvars;
use crate::hir_maker::HirMaker;
use crate::hir_maker_context::*;
use crate::pattern_match;
use crate::type_checking;
use anyhow::Result;
use shiika_ast::Token;
use shiika_ast::*;
use shiika_core::{names::*, ty, ty::*};
use skc_hir::*;
use std::collections::HashMap;

/// Result of looking up a lvar
#[derive(Debug)]
struct LVarInfo {
    ty: TermTy,
    detail: LVarDetail,
}
#[derive(Debug)]
enum LVarDetail {
    /// Found in the current scope
    CurrentScope { name: String },
    /// Found in the current method/lambda argument
    Argument { idx: usize },
    /// Found in outer scope
    OuterScope {
        /// Index of the lvar in `captures`
        cidx: usize,
        readonly: bool,
    },
}

impl LVarInfo {
    /// Returns HirExpression to refer this lvar
    fn ref_expr(&self) -> HirExpression {
        match &self.detail {
            LVarDetail::CurrentScope { name } => Hir::lvar_ref(self.ty.clone(), name.clone()),
            LVarDetail::Argument { idx } => Hir::arg_ref(self.ty.clone(), *idx),
            LVarDetail::OuterScope { cidx, readonly } => {
                Hir::lambda_capture_ref(self.ty.clone(), *cidx, *readonly)
            }
        }
    }

    /// Returns HirExpression to update this lvar
    fn assign_expr(&self, expr: HirExpression) -> HirExpression {
        match &self.detail {
            LVarDetail::CurrentScope { name, .. } => Hir::lvar_assign(name, expr),
            LVarDetail::Argument { .. } => panic!("[BUG] Cannot reassign argument"),
            LVarDetail::OuterScope { cidx, .. } => Hir::lambda_capture_write(*cidx, expr),
        }
    }
}

impl<'hir_maker> HirMaker<'hir_maker> {
    pub(super) fn convert_exprs(&mut self, exprs: &[AstExpression]) -> Result<HirExpressions> {
        let hir_exprs = exprs
            .iter()
            .map(|expr| self.convert_expr(expr))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(HirExpressions::new(hir_exprs))
    }

    pub(super) fn convert_expr(&mut self, expr: &AstExpression) -> Result<HirExpression> {
        match &expr.body {
            AstExpressionBody::LogicalNot { expr } => self.convert_logical_not(expr),
            AstExpressionBody::LogicalAnd { left, right } => self.convert_logical_and(left, right),
            AstExpressionBody::LogicalOr { left, right } => self.convert_logical_or(left, right),
            AstExpressionBody::If {
                cond_expr,
                then_exprs,
                else_exprs,
            } => self.convert_if_expr(cond_expr, then_exprs, else_exprs),

            AstExpressionBody::Match { cond_expr, clauses } => {
                self.convert_match_expr(cond_expr, clauses)
            }

            AstExpressionBody::While {
                cond_expr,
                body_exprs,
            } => self.convert_while_expr(cond_expr, body_exprs),

            AstExpressionBody::Break => self.convert_break_expr(),

            AstExpressionBody::Return { arg } => self.convert_return_expr(arg),

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
                type_args,
                ..
            } => self.convert_method_call(receiver_expr, method_name, arg_exprs, type_args),

            AstExpressionBody::LambdaExpr {
                params,
                exprs,
                is_fn,
            } => self.convert_lambda_expr(params, exprs, is_fn),

            AstExpressionBody::BareName(name) => self.convert_bare_name(name),

            AstExpressionBody::IVarRef(names) => self.convert_ivar_ref(names),

            AstExpressionBody::ConstRef(names) => self.convert_const_ref(names),

            AstExpressionBody::SpecializeExpression { base_name, args } => {
                self.convert_specialize_expr(base_name, args)
            }

            AstExpressionBody::PseudoVariable(token) => self.convert_pseudo_variable(token),

            AstExpressionBody::ArrayLiteral(exprs) => self.convert_array_literal(exprs),

            AstExpressionBody::FloatLiteral { value } => Ok(Hir::float_literal(*value)),

            AstExpressionBody::DecimalLiteral { value } => Ok(Hir::decimal_literal(*value)),

            AstExpressionBody::StringLiteral { content } => {
                Ok(self.convert_string_literal(content))
            } //x => panic!("TODO: {:?}", x)
        }
    }

    fn convert_logical_not(&mut self, expr: &AstExpression) -> Result<HirExpression> {
        let expr_hir = self.convert_expr(expr)?;
        type_checking::check_logical_operator_ty(&expr_hir.ty, "argument of logical not")?;
        Ok(Hir::logical_not(expr_hir))
    }

    fn convert_logical_and(
        &mut self,
        left: &AstExpression,
        right: &AstExpression,
    ) -> Result<HirExpression> {
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
    ) -> Result<HirExpression> {
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
    ) -> Result<HirExpression> {
        let cond_hir = self.convert_expr(cond_expr)?;
        type_checking::check_condition_ty(&cond_hir.ty, "if")?;

        let mut then_hirs = self.convert_exprs(then_exprs)?;
        let mut else_hirs = match else_exprs {
            Some(exprs) => self.convert_exprs(exprs)?,
            None => HirExpressions::new(vec![]),
        };

        let if_ty = if then_hirs.ty.is_never_type() {
            else_hirs.ty.clone()
        } else if else_hirs.ty.is_never_type() {
            then_hirs.ty.clone()
        } else if then_hirs.ty.is_void_type() {
            else_hirs.voidify();
            ty::raw("Void")
        } else if else_hirs.ty.is_void_type() {
            then_hirs.voidify();
            ty::raw("Void")
        } else {
            let opt_ty = self
                .class_dict
                .nearest_common_ancestor(&then_hirs.ty, &else_hirs.ty);
            let ty = type_checking::check_if_body_ty(opt_ty)?;
            if !then_hirs.ty.equals_to(&ty) {
                then_hirs = then_hirs.bitcast_to(ty.clone());
            }
            if !else_hirs.ty.equals_to(&ty) {
                else_hirs = else_hirs.bitcast_to(ty.clone());
            }
            ty
        };

        Ok(Hir::if_expression(if_ty, cond_hir, then_hirs, else_hirs))
    }

    fn convert_match_expr(
        &mut self,
        cond_expr: &AstExpression,
        clauses: &[AstMatchClause],
    ) -> Result<HirExpression> {
        let (match_expr, lvars) = pattern_match::convert_match_expr(self, cond_expr, clauses)?;
        for (name, ty) in lvars {
            let readonly = true;
            self.ctx_stack.declare_lvar(&name, ty, readonly);
        }
        Ok(match_expr)
    }

    fn convert_while_expr(
        &mut self,
        cond_expr: &AstExpression,
        body_exprs: &[AstExpression],
    ) -> Result<HirExpression> {
        let cond_hir = self.convert_expr(cond_expr)?;
        type_checking::check_condition_ty(&cond_hir.ty, "while")?;

        self.ctx_stack.push(HirMakerContext::while_ctx());
        let body_hirs = self.convert_exprs(body_exprs)?;
        self.ctx_stack.pop_while_ctx();

        Ok(Hir::while_expression(cond_hir, body_hirs))
    }

    fn convert_break_expr(&mut self) -> Result<HirExpression> {
        let from;
        match self.ctx_stack.top_mut() {
            HirMakerContext::Lambda(lambda_ctx) => {
                if lambda_ctx.is_fn {
                    return Err(error::program_error("`break' inside a fn"));
                } else {
                    // OK for now. This `break` still may be invalid
                    // (eg. `ary.map{ break }`) but it cannot be checked here
                    lambda_ctx.has_break = true;
                    from = HirBreakFrom::Block;
                }
            }
            HirMakerContext::While(_) => {
                from = HirBreakFrom::While;
            }
            _ => {
                return Err(error::program_error("`break' outside a loop"));
            }
        }
        Ok(Hir::break_expression(from))
    }

    fn convert_return_expr(&mut self, arg: &Option<Box<AstExpression>>) -> Result<HirExpression> {
        let from = self._validate_return()?;
        let arg_expr = if let Some(x) = arg {
            self.convert_expr(x)?
        } else {
            Hir::const_ref(ty::raw("Void"), toplevel_const("Void"))
        };
        self._validate_return_type(&arg_expr.ty)?;
        Ok(Hir::return_expression(from, arg_expr))
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
    fn _validate_return_type(&self, arg_ty: &TermTy) -> Result<()> {
        if self.ctx_stack.lambda_ctx().is_some() {
            // TODO: check arg_ty matches to fn's return type
        } else if let Some(method_ctx) = &self.ctx_stack.method_ctx() {
            type_checking::check_return_arg_type(&self.class_dict, arg_ty, &method_ctx.signature)?;
        }
        Ok(())
    }

    fn convert_lvar_assign(
        &mut self,
        name: &str,
        rhs: &AstExpression,
        is_var: &bool,
    ) -> Result<HirExpression> {
        let expr = self.convert_expr(rhs)?;
        // For `var x`, `x` should not be exist
        if *is_var && self._lookup_var(name).is_some() {
            return Err(error::program_error(&format!(
                "variable `{}' already exists",
                name
            )));
        }
        if let Some(mut lvar_info) = self._find_var(name, true)? {
            // Reassigning
            if lvar_info.ty != expr.ty {
                if let Some(t) = self
                    .class_dict
                    .nearest_common_ancestor(&lvar_info.ty, &expr.ty)
                {
                    // Upgrade lvar type (eg. from `None` to `Maybe<Int>`)
                    lvar_info.ty = t;
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
            // Create new lvar
            self.ctx_stack.declare_lvar(name, expr.ty.clone(), !is_var);
            Ok(Hir::lvar_assign(name, expr))
        }
    }

    fn convert_ivar_assign(
        &mut self,
        name: &str,
        rhs: &AstExpression,
        is_var: &bool,
    ) -> Result<HirExpression> {
        let expr = self.convert_expr(rhs)?;
        let base_ty = self.ctx_stack.self_ty().erasure_ty();

        if self.ctx_stack.in_initializer() {
            let idx = self.declare_ivar(name, &expr.ty, !is_var)?;
            return Ok(Hir::ivar_assign(name, idx, expr, *is_var, base_ty));
        }

        if let Some(ivar) = self.class_dict.find_ivar(&base_ty.fullname, name) {
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
            Ok(Hir::ivar_assign(name, ivar.idx, expr, false, base_ty))
        } else {
            Err(error::program_error(&format!(
                "instance variable `{}' not found",
                name
            )))
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
    ) -> Result<HirExpression> {
        // TODO: forbid `A::B = 1`
        let fullname = toplevel_const(&names.join("::"));
        let hir_expr = self.convert_expr(rhs)?;
        self.constants.insert(fullname.clone(), hir_expr.ty.clone());
        Ok(Hir::const_assign(fullname, hir_expr))
    }

    fn convert_method_call(
        &mut self,
        receiver_expr: &Option<Box<AstExpression>>,
        method_name: &MethodFirstname,
        arg_exprs: &[AstExpression],
        type_args: &[AstExpression],
    ) -> Result<HirExpression> {
        let arg_hirs = arg_exprs
            .iter()
            .map(|arg_expr| self.convert_expr(arg_expr))
            .collect::<Result<Vec<_>, _>>()?;

        // Check if this is a lambda invocation
        if receiver_expr.is_none() {
            if let Some(lvar) = self._lookup_var(&method_name.0) {
                if let Some(ret_ty) = lvar.ty.fn_x_info() {
                    return Ok(Hir::lambda_invocation(ret_ty, lvar.ref_expr(), arg_hirs));
                }
            }
        }

        let receiver_hir = match receiver_expr {
            Some(expr) => self.convert_expr(expr)?,
            // Implicit self
            _ => self.convert_self_expr(),
        };
        let mut method_tyargs = vec![];
        for arg in type_args {
            method_tyargs.push(self._resolve_method_tyarg(arg)?);
        }
        let (sig, found_class_name) = self.class_dict.lookup_method(
            &receiver_hir.ty,
            method_name,
            method_tyargs.as_slice(),
        )?;
        self._make_method_call(receiver_hir, arg_hirs, sig, found_class_name)
    }

    /// Resolve a method tyarg (a ConstName) into a TermTy
    /// eg.
    ///     ary.map<Array<T>>(f)
    ///             ~~~~~~~~
    ///             => TermTy(Array<TyParamRef(T)>)
    fn _resolve_method_tyarg(&mut self, arg: &AstExpression) -> Result<TermTy> {
        match &arg.body {
            AstExpressionBody::ConstRef(name) => Ok(self.resolve_class_const(name)?.0),
            AstExpressionBody::SpecializeExpression { base_name, args } => Ok(self
                .convert_specialize_expr(base_name, args)?
                .ty
                .instance_ty()),
            _ => panic!("[BUG] unexpected arg in method tyargs"),
        }
    }

    /// Check the arguments and create HirMethodCall
    fn _make_method_call(
        &self,
        receiver_hir: HirExpression,
        mut arg_hirs: Vec<HirExpression>,
        sig: MethodSignature,
        found_class_name: ClassFullname,
    ) -> Result<HirExpression> {
        let specialized = receiver_hir.ty.is_specialized();
        let arg_tys = arg_hirs.iter().map(|expr| &expr.ty).collect::<Vec<_>>();
        type_checking::check_method_args(
            &self.class_dict,
            &sig,
            &arg_tys,
            &receiver_hir,
            &arg_hirs,
        )?;
        if let Some(last_arg) = arg_hirs.last_mut() {
            check_break_in_block(&sig, last_arg)?;
        }

        let receiver = if found_class_name != receiver_hir.ty.fullname {
            // Upcast needed
            Hir::bit_cast(found_class_name.instance_ty(), receiver_hir)
        } else {
            receiver_hir
        };

        let args = if specialized {
            arg_hirs
                .into_iter()
                .map(|expr| Hir::bit_cast(ty::raw("Object"), expr))
                .collect::<Vec<_>>()
        } else {
            arg_hirs
        };

        let mut ret = Hir::method_call(sig.ret_ty.clone(), receiver, sig.fullname.clone(), args);
        if specialized {
            ret = Hir::bit_cast(sig.ret_ty, ret)
        }
        Ok(ret)
    }

    pub(super) fn convert_lambda_expr(
        &mut self,
        params: &[shiika_ast::Param],
        exprs: &[AstExpression],
        is_fn: &bool,
    ) -> Result<HirExpression> {
        let namespace = self.ctx_stack.const_scopes().next().unwrap();
        let hir_params = self.class_dict.convert_params(
            &namespace,
            params,
            &self.ctx_stack.current_class_typarams(),
            &self.ctx_stack.current_method_typarams(),
        )?;

        // Convert lambda body
        self.ctx_stack
            .push(HirMakerContext::lambda(*is_fn, hir_params.clone()));
        let hir_exprs = self.convert_exprs(exprs)?;
        let mut lambda_ctx = self.ctx_stack.pop_lambda_ctx();
        Ok(Hir::lambda_expr(
            lambda_ty(&hir_params, &hir_exprs.ty),
            self.create_lambda_name(),
            hir_params,
            hir_exprs,
            self._resolve_lambda_captures(lambda_ctx.captures), // hir_captures
            extract_lvars(&mut lambda_ctx.lvars),               // lvars
            lambda_ctx.has_break,
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
    /// Also, concat lambda_captures to outer_captures
    fn _resolve_lambda_captures(
        &mut self,
        lambda_captures: Vec<LambdaCapture>,
    ) -> Vec<HirLambdaCapture> {
        let mut ret = vec![];
        for cap in lambda_captures {
            let captured_here = if let HirMakerContext::Lambda(_) = self.ctx_stack.top() {
                matches!(cap.ctx_depth, Some(idx) if idx == self.ctx_stack.len() - 1)
            } else {
                cap.ctx_depth.is_none()
            };
            if captured_here {
                // The variable is in this scope
                match cap.detail {
                    LambdaCaptureDetail::CapLVar { name } => {
                        ret.push(HirLambdaCapture::CaptureLVar { name });
                    }
                    LambdaCaptureDetail::CapFnArg { idx } => {
                        ret.push(HirLambdaCapture::CaptureArg { idx });
                    }
                }
            } else {
                // The variable is in outer scope
                let ty = cap.ty.clone();
                let cidx = self.ctx_stack.push_lambda_capture(cap);
                ret.push(HirLambdaCapture::CaptureFwd { cidx, ty });
            }
        }
        ret
    }

    /// Generate local variable reference or method call with implicit receiver(self)
    fn convert_bare_name(&mut self, name: &str) -> Result<HirExpression> {
        // Found a local variable
        if let Some(lvar_info) = self._find_var(name, false)? {
            return Ok(lvar_info.ref_expr());
        }

        // Search method
        let self_expr = self.convert_self_expr();
        let found = self
            .class_dict
            .lookup_method(&self_expr.ty, &method_firstname(name), &[]);
        if let Ok((sig, found_class_name)) = found {
            self._make_method_call(self_expr, vec![], sig, found_class_name)
        } else {
            Err(error::program_error(&format!(
                "variable or method `{}' was not found",
                name
            )))
        }
    }

    /// Return the variable of the given name, if any
    fn _lookup_var(&mut self, name: &str) -> Option<LVarInfo> {
        self._find_var(name, false).unwrap()
    }

    /// Find the variable of the given name.
    /// If it is a free variable, lambda_ctx.captures will be modified
    fn _find_var(&mut self, name: &str, updating: bool) -> Result<Option<LVarInfo>> {
        let (in_lambda, cidx) = if let Some(lambda_ctx) = self.ctx_stack.innermost_lambda() {
            (true, lambda_ctx.captures.len())
        } else {
            (false, 0)
        };
        let (found, opt_cap) = self.__find_var(in_lambda, cidx, name, updating)?;
        if let Some(cap) = opt_cap {
            self.ctx_stack.push_lambda_capture(cap);
        }
        Ok(found)
    }

    fn __find_var(
        &mut self,
        in_lambda: bool,
        cidx: usize,
        name: &str,
        updating: bool,
    ) -> Result<(Option<LVarInfo>, Option<LambdaCapture>)> {
        let mut lambda_seen = false;
        for (lvars, params, opt_depth) in self.ctx_stack.lvar_scopes() {
            let is_lambda_capture = in_lambda && lambda_seen;
            if let Some(lvar) = lvars.get(name) {
                if updating && lvar.readonly {
                    return Err(error::program_error(&format!(
                        "cannot reassign to `{}' (Hint: declare it with `var')",
                        name
                    )));
                }
                if is_lambda_capture {
                    let cap = LambdaCapture {
                        ctx_depth: opt_depth,
                        ty: lvar.ty.clone(),
                        detail: LambdaCaptureDetail::CapLVar {
                            name: name.to_string(),
                        },
                    };
                    let lvar_info = LVarInfo {
                        ty: lvar.ty.clone(),
                        detail: LVarDetail::OuterScope {
                            cidx,
                            readonly: false,
                        },
                    };
                    return Ok((Some(lvar_info), Some(cap)));
                } else {
                    let lvar_info = LVarInfo {
                        ty: lvar.ty.clone(),
                        detail: LVarDetail::CurrentScope {
                            name: name.to_string(),
                        },
                    };
                    return Ok((Some(lvar_info), None));
                }
            }
            if let Some((idx, param)) = signature::find_param(params, name) {
                if updating {
                    return Err(error::program_error(&format!(
                        "you cannot reassign to argument `{}'",
                        name
                    )));
                }
                if is_lambda_capture {
                    let cap = LambdaCapture {
                        ctx_depth: opt_depth,
                        ty: param.ty.clone(),
                        detail: LambdaCaptureDetail::CapFnArg { idx },
                    };
                    let lvar_info = LVarInfo {
                        ty: param.ty.clone(),
                        detail: LVarDetail::OuterScope {
                            cidx,
                            readonly: true,
                        },
                    };
                    return Ok((Some(lvar_info), Some(cap)));
                } else {
                    let lvar_info = LVarInfo {
                        ty: param.ty.clone(),
                        detail: LVarDetail::Argument { idx },
                    };
                    return Ok((Some(lvar_info), None));
                }
            }
            let is_lambda_scope = opt_depth.is_some();
            if is_lambda_scope {
                lambda_seen = true;
            }
        }
        Ok((None, None))
    }

    fn convert_ivar_ref(&self, name: &str) -> Result<HirExpression> {
        let base_ty = self.ctx_stack.self_ty().erasure_ty();
        let found = self
            .class_dict
            .find_ivar(&base_ty.fullname, name)
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
            )),
            None => Err(error::program_error(&format!(
                "ivar `{}' was not found",
                name
            ))),
        }
    }

    /// Resolve constant name
    /// Also, register specialized class constant and its type eg. for
    /// `Maybe<Int>`, constant `Maybe<Int>` and type `Meta:Maybe<Int>`
    fn convert_const_ref(&mut self, name: &UnresolvedConstName) -> Result<HirExpression> {
        let (ty, resolved) = self._resolve_simple_const(name)?;
        Ok(Hir::const_ref(ty, resolved.to_const_fullname()))
    }

    /// Resolve a constant name without tyargs
    fn _resolve_simple_const(
        &self,
        name: &UnresolvedConstName,
    ) -> Result<(TermTy, ResolvedConstName)> {
        if name.0.len() == 1 {
            // Check if it is a typaram ref
            let s = name.0.first().unwrap();
            if let Some(ty) = self.ctx_stack.lookup_typaram(s) {
                return Ok((ty, typaram_as_resolved_const_name(s)));
            }
        }
        for namespace in self.ctx_stack.const_scopes() {
            let resolved = resolved_const_name(namespace, name.0.to_vec());
            let full = resolved.to_const_fullname();
            if let Some(ty) = self._lookup_const(&full) {
                return Ok((ty, resolved));
            }
        }
        Err(error::program_error(&format!(
            "constant `{:?}' was not found",
            name.0.join("::")
        )))
    }

    /// Resolve const name which *must be* a class
    pub(super) fn resolve_class_const(
        &self,
        name: &UnresolvedConstName,
    ) -> Result<(TermTy, ResolvedConstName)> {
        let (resolved_ty, resolved_name) = self._resolve_simple_const(name)?;

        // `ty` is `Meta:XX` here but we want to remove `Meta:`
        // unless `ty` is typaram ref
        let ty = if resolved_ty.is_typaram_ref() {
            resolved_ty
        } else if resolved_ty.is_metaclass() {
            resolved_ty.instance_ty()
        } else {
            return Err(error::type_error(&format!(
                "{} is not a class but {}",
                resolved_name, resolved_ty
            )));
        };
        Ok((ty, resolved_name))
    }

    /// Check if a constant is registered
    fn _lookup_const(&self, full: &ConstFullname) -> Option<TermTy> {
        self.constants
            .get(full)
            .or_else(|| self.imported_constants.get(full))
            .cloned()
    }

    /// Create `Meta:A<B>` for type `A<B>` (unless already exists)
    pub fn create_specialized_meta_class(&mut self, meta_ty: &TermTy) {
        debug_assert!(meta_ty.is_metaclass());
        if self.class_dict.lookup_class(&meta_ty.fullname).is_some() {
            return;
        }
        let meta_base = meta_ty.erasure(); // `Meta:A`
        let mut ivars = HashMap::new();
        ivars.insert(
            "name".to_string(),
            SkIVar {
                idx: 0,
                name: "name".to_string(),
                ty: ty::raw("String"),
                readonly: true,
            },
        );
        let metacls = self
            .class_dict
            .get_class(&meta_base)
            .specialized_meta(meta_ty.tyargs());
        self.class_dict.add_class(metacls);
    }

    fn convert_specialize_expr(
        &mut self,
        base_name: &UnresolvedConstName,
        args: &[AstExpression],
    ) -> Result<HirExpression> {
        debug_assert!(!args.is_empty());
        let (base_ty, _) = self.resolve_class_const(base_name)?;
        let mut type_args = vec![];
        for arg in args {
            let ty = match &arg.body {
                AstExpressionBody::ConstRef(n) => self.resolve_class_const(n)?.0,
                AstExpressionBody::SpecializeExpression {
                    base_name: n,
                    args: a,
                } => self.convert_specialize_expr(n, a)?.ty.instance_ty(),
                _ => panic!("[BUG] unexpected arg in SpecializeExpression"),
            };
            type_args.push(ty);
        }
        let ty = ty::spe(base_ty.fullname.0, type_args);
        let full = const_fullname(&ty.fullname.0);
        let meta_ty = ty.meta_ty();
        if self._lookup_const(&full).is_none() {
            self._register_specialized_const(&meta_ty, &full);
        }
        Ok(Hir::const_ref(meta_ty, full))
    }

    /// Register specialized class constant and its type.
    /// eg. for `Maybe<Int>`, constant `Maybe<Int>` and type `Meta:Maybe<Int>`
    fn _register_specialized_const(&mut self, meta_ty: &TermTy, full: &ConstFullname) {
        debug_assert!(meta_ty.is_metaclass());
        // For `A<B>`, create its type `Meta:A<B>`
        self.create_specialized_meta_class(&meta_ty);
        // Register const `A<B>`
        let str_idx = self.register_string_literal(&full.0);
        let expr = Hir::class_literal(
            meta_ty.clone(),
            meta_ty.instance_ty().fullname.clone(),
            str_idx,
        );
        self.register_const_full(full.clone(), expr);
    }

    fn convert_pseudo_variable(&self, token: &Token) -> Result<HirExpression> {
        match token {
            Token::KwSelf => Ok(self.convert_self_expr()),
            Token::KwTrue => Ok(Hir::boolean_literal(true)),
            Token::KwFalse => Ok(Hir::boolean_literal(false)),
            _ => panic!("[BUG] not a pseudo variable token: {:?}", token),
        }
    }

    /// Generate HIR for an array literal
    /// `[x,y]` is expanded into `tmp = Array<Object>.new; tmp.push(x); tmp.push(y)`
    fn convert_array_literal(&mut self, item_exprs: &[AstExpression]) -> Result<HirExpression> {
        let item_exprs = item_exprs
            .iter()
            .map(|expr| self.convert_expr(expr))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(self.convert_array_literal_(item_exprs))
    }

    fn convert_array_literal_(&mut self, item_exprs: Vec<HirExpression>) -> HirExpression {
        // TODO #102: Support empty array literal
        let mut item_ty = if item_exprs.is_empty() {
            ty::raw("Object")
        } else {
            item_exprs[0].ty.clone()
        };

        for expr in &item_exprs {
            item_ty = self
                .class_dict
                .nearest_common_ancestor(&item_ty, &expr.ty)
                .expect("array literal elements type mismatch");
        }
        let ary_ty = ty::spe("Array", vec![item_ty]);

        Hir::array_literal(item_exprs, ary_ty)
    }

    fn convert_self_expr(&self) -> HirExpression {
        Hir::self_expression(self.ctx_stack.self_ty())
    }

    fn convert_string_literal(&mut self, content: &str) -> HirExpression {
        let idx = self.register_string_literal(content);
        Hir::string_literal(idx)
    }

    pub(super) fn register_string_literal(&mut self, content: &str) -> usize {
        let idx = self.str_literals.len();
        self.str_literals.push(content.to_string());
        idx
    }
}

fn lambda_ty(params: &[MethodParam], ret_ty: &TermTy) -> TermTy {
    let mut tyargs = params.iter().map(|x| x.ty.clone()).collect::<Vec<_>>();
    tyargs.push(ret_ty.clone());
    ty::spe(&format!("Fn{}", params.len()), tyargs)
}

/// Check if `break` in block is valid
fn check_break_in_block(sig: &MethodSignature, last_arg: &mut HirExpression) -> Result<()> {
    if let HirExpressionBase::HirLambdaExpr { has_break, .. } = last_arg.node {
        if has_break {
            if sig.ret_ty == ty::raw("Void") {
                match &mut last_arg.node {
                    HirExpressionBase::HirLambdaExpr { ret_ty, .. } => {
                        std::mem::swap(ret_ty, &mut ty::raw("Void"));
                    }
                    _ => panic!("[BUG] unexpected type"),
                }
            } else {
                return Err(error::program_error(
                    "`break' not allowed because this block is expected to return a value",
                ));
            }
        }
    }
    Ok(())
}
