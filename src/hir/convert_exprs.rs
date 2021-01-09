use crate::ast::*;
use crate::error;
use crate::error::Error;
use crate::hir::hir_maker::HirMaker;
use crate::hir::hir_maker_context::*;
use crate::hir::*;
use crate::parser::token::Token;
use crate::type_checking;

#[derive(Debug)]
enum LVarInfo {
    CurrentScope {
        ty: TermTy,
        name: String,
    },
    Argument {
        ty: TermTy,
        idx: usize,
    },
    OuterScope {
        ty: TermTy,
        arity: usize,
        cidx: usize,
        readonly: bool,
    },
}

impl LVarInfo {
    fn ref_expr(&self) -> HirExpression {
        match self {
            LVarInfo::CurrentScope { ty, name } => Hir::lvar_ref(ty.clone(), name.clone()),
            LVarInfo::Argument { ty, idx } => Hir::arg_ref(ty.clone(), *idx),
            LVarInfo::OuterScope {
                ty, cidx, readonly, ..
            } => Hir::lambda_capture_ref(ty.clone(), *cidx, *readonly),
        }
    }

    fn assign_expr(&self, expr: HirExpression) -> HirExpression {
        match self {
            LVarInfo::CurrentScope { name, .. } => Hir::lvar_assign(name, expr),
            LVarInfo::Argument { .. } => panic!("[BUG] Cannot reassign argument"),
            LVarInfo::OuterScope { arity, cidx, .. } => {
                Hir::lambda_capture_write(*arity, *cidx, expr)
            }
        }
    }
}

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

            AstExpressionBody::ConstRef(name) => self.convert_const_ref(name),

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

        let mut then_hirs = self.convert_exprs(then_exprs)?;
        let mut else_hirs = match else_exprs {
            Some(exprs) => self.convert_exprs(exprs)?,
            None => HirExpressions::new(vec![]),
        };

        if then_hirs.ty.is_void_type() && !else_hirs.ty.is_never_type() {
            else_hirs.voidify();
        } else if else_hirs.ty.is_void_type() && !then_hirs.ty.is_never_type() {
            then_hirs.voidify();
        } else {
            type_checking::check_if_clauses_ty(&then_hirs.ty, &else_hirs.ty)?;
        }

        let if_ty = if then_hirs.ty.is_never_type() { else_hirs.ty.clone() }
          else { then_hirs.ty.clone() };

        Ok(Hir::if_expression(
            if_ty,
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
        let existing_lvar = self._lookup_var(name, true)?;
        let ctx = self.ctx_mut();
        // REFACTOR: since we have `existing_lvar` now, we don't need to see `ctx.lvars` here.
        match ctx.lvars.get(name) {
            Some(lvar) => {
                // Reassigning
                if lvar.readonly {
                    return Err(error::program_error(&format!(
                        "cannot reassign to `{}' (Hint: declare it with `var')",
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
                // Check it is defined in outer scope
                if let Some(lvar_info) = existing_lvar {
                    return Ok(lvar_info.assign_expr(expr));
                } else {
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
        }

        Ok(Hir::lvar_assign(name, expr))
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
            let self_ty = ctx.self_ty.clone();
            let idx = self.declare_ivar(name, &expr.ty, !is_var)?;
            return Ok(Hir::ivar_assign(name, idx, expr, *is_var, self_ty));
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
            Ok(Hir::ivar_assign(name, ivar.idx, expr, false, ctx.self_ty.clone()))
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
        Ok(Hir::const_assign(fullname, self.convert_expr(rhs)?))
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
        let arg_hirs = arg_exprs
            .iter()
            .map(|arg_expr| self.convert_expr(arg_expr))
            .collect::<Result<Vec<_>, _>>()?;
        self.make_method_call(receiver_hir, &method_name, arg_hirs)
    }

    /// Resolve the method and create HirMethodCall
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
        type_checking::check_method_args(
            &self.class_dict,
            &sig,
            &param_tys,
            &receiver_hir,
            &arg_hirs,
        )?;

        let receiver = if &found_class_name != class_fullname {
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

    fn convert_lambda_expr(
        &mut self,
        params: &[ast::Param],
        exprs: &[AstExpression],
    ) -> Result<HirExpression, Error> {
        self.lambda_ct += 1;
        let lambda_id = self.lambda_ct;
        let hir_params = signature::convert_params(params, &self.current_class_typarams());

        // Convert lambda body
        self.push_ctx(HirMakerContext::lambda_ctx(self.ctx(), hir_params.clone()));
        let hir_exprs = self.convert_exprs(exprs)?;
        let mut lambda_ctx = self.pop_ctx();

        let lvars = lambda_ctx.extract_lvars();
        let captures = self._resolve_lambda_captures(lambda_ctx.captures);

        let name = format!("lambda_{}", lambda_id);
        let ty = lambda_ty(&hir_params, &hir_exprs.ty);
        Ok(Hir::lambda_expr(
            ty, name, hir_params, hir_exprs, captures, lvars,
        ))
    }

    /// Resolve LambdaCapture into HirExpression
    /// Also, concat lambda_captures to outer_captures
    fn _resolve_lambda_captures(&mut self, captures: Vec<LambdaCapture>) -> Vec<HirLambdaCapture> {
        let ctx = self.ctx_mut();
        captures
            .into_iter()
            .map(|cap| {
                if cap.ctx_depth == ctx.depth {
                    // The variable is in this scope
                    match cap.detail {
                        LambdaCaptureDetail::CapLVar { name } => {
                            HirLambdaCapture::CaptureLVar { name }
                        }
                        LambdaCaptureDetail::CapFnArg { idx } => {
                            HirLambdaCapture::CaptureArg { idx }
                        }
                    }
                } else {
                    // The variable is in outer scope
                    let ty = cap.ty.clone();
                    ctx.captures.push(cap);
                    let cidx = ctx.captures.len() - 1;
                    HirLambdaCapture::CaptureFwd { cidx, ty }
                }
            })
            .collect()
    }

    /// Generate local variable reference or method call with implicit receiver(self)
    fn convert_bare_name(&mut self, name: &str) -> Result<HirExpression, Error> {
        if let Some(lvar_info) = self._lookup_var(name, false)? {
            Ok(lvar_info.ref_expr())
        } else {
            Err(error::program_error(&format!(
                "variable `{}' was not found",
                name
            )))
        }
    }

    /// Lookup variable of the given name.
    /// If it is a free variable, ctx.captures will be modified
    /// If `updating` is true, readonly variables are skipped
    fn _lookup_var(&mut self, name: &str, updating: bool) -> Result<Option<LVarInfo>, Error> {
        let ctx = self.ctx();
        // Local
        if let Some(lvar) = ctx.find_lvar(name) {
            if !updating || !lvar.readonly {
                return Ok(Some(LVarInfo::CurrentScope {
                    ty: lvar.ty.clone(),
                    name: name.to_string(),
                }));
            }
        }
        // Arg
        if let Some((idx, param)) = ctx.find_fn_arg(name) {
            if updating {
                return Err(error::program_error(&format!(
                    "you cannot reassign to argument `{}'",
                    name
                )));
            } else {
                return Ok(Some(LVarInfo::Argument {
                    ty: param.ty.clone(),
                    idx,
                }));
            }
        }
        // Outer
        if let Some(outer_ctx) = self.outer_lvar_scope_of(&ctx) {
            // The `ctx` has outer scope == `ctx` is a lambda
            let arity = ctx.method_sig.as_ref().unwrap().params.len();
            let cidx = ctx.captures.len();
            if let Some((cap, lvar_info)) =
                self.lookup_var_in_outer_scope(arity, cidx, outer_ctx, name, updating)?
            {
                self.ctx_mut().captures.push(cap);
                return Ok(Some(lvar_info));
            }
        }
        Ok(None)
    }

    /// Lookup variable of the given name in the outer scopes.
    /// Return a `LambdaCapture` (which variable is captured) and a
    /// `HirExpression` (how it can be retrieved from `captures`).
    fn lookup_var_in_outer_scope(
        &self,
        arity: usize,
        cidx: usize,
        ctx: &HirMakerContext,
        name: &str,
        updating: bool,
    ) -> Result<Option<(LambdaCapture, LVarInfo)>, Error> {
        // Check local var
        if let Some(lvar) = ctx.find_lvar(name) {
            if updating && lvar.readonly {
                return Err(error::program_error(&format!(
                    "cannot reassign to `{}' (Hint: declare it with `var')",
                    name
                )));
            }
            let cap = LambdaCapture {
                ctx_depth: ctx.depth,
                ty: lvar.ty.clone(),
                detail: LambdaCaptureDetail::CapLVar {
                    name: name.to_string(),
                },
            };
            return Ok(Some((
                cap,
                LVarInfo::OuterScope {
                    ty: lvar.ty.clone(),
                    arity,
                    cidx,
                    readonly: false,
                },
            )));
        }
        // Check argument
        if let Some((idx, param)) = ctx.find_fn_arg(name) {
            if updating {
                return Err(error::program_error(&format!(
                    "you cannot reassign to argument `{}'",
                    name
                )));
            }
            let cap = LambdaCapture {
                ctx_depth: ctx.depth,
                ty: param.ty.clone(),
                detail: LambdaCaptureDetail::CapFnArg { idx },
            };
            return Ok(Some((
                cap,
                LVarInfo::OuterScope {
                    ty: param.ty.clone(),
                    arity,
                    cidx,
                    readonly: true,
                },
            )));
        }

        // TODO: It may be a nullary method call

        // Lookup in the next surrounding context
        if let Some(outer_ctx) = self.outer_lvar_scope_of(ctx) {
            self.lookup_var_in_outer_scope(arity, cidx, &*outer_ctx, name, updating)
        } else {
            Ok(None)
        }
    }

    fn convert_ivar_ref(&self, name: &str) -> Result<HirExpression, Error> {
        let method_ctx = self.method_ctx().ok_or_else(|| {
            error::program_error(&format!("referring ivar `{}' out of a method", name))
        })?;
        let self_cls = &method_ctx.self_ty.fullname;
        let found = self.class_dict.find_ivar(&self_cls, name).or_else(||{
            method_ctx.iivars.get(name)
        });
        match found {
            Some(ivar) => Ok(Hir::ivar_ref(ivar.ty.clone(), name.to_string(), ivar.idx, self.ctx().self_ty.clone())),
            None => Err(error::program_error(&format!(
                "ivar `{}' was not found",
                name
            ))),
        }
    }

    /// Resolve constant name
    fn convert_const_ref(&mut self, name: &ConstName) -> Result<HirExpression, Error> {
        if let Some((ty, fullname)) = self._lookup_const(name) {
            return Ok(Hir::const_ref(ty.clone(), fullname));
        }
        // Check if it refers to a class
        self._check_class_exists(name)?;
        let class_ty = self._create_class_const(name);
        Ok(Hir::const_ref(class_ty, name.to_const_fullname()))
    }

    /// Lookup a constant from current scope
    fn _lookup_const(&self, name: &ConstName) -> Option<(&TermTy, ConstFullname)> {
        let fullname = name.to_const_fullname();
        if let Some(found) = self.constants.get(&fullname) {
            return Some((found, fullname))
        }

        let fullname = name.under_namespace(&(self.ctx().namespace.0.clone()));
        self.constants.get(&fullname).map(|found| {
            (found, fullname)
        })
    }

    /// Check `name` refers proper class name
    /// eg. For `A<B<C>>`, check A, B and C exists
    fn _check_class_exists(&self, name: &ConstName) -> Result<(), Error> {
        if !self.class_dict.class_exists(&name.names.join("::")) {
            return Err(error::program_error(&format!(
                "constant `{:?}' was not found",
                name.names.join("::")
            )))
        }
        if name.args.is_empty() {
            return Ok(())
        }
        let mut typarams = &vec![];
        if let Some(class_ctx) = self.class_ctx() {
            typarams = &class_ctx.typarams
        }
        for arg in &name.args {
            if typarams.contains(&arg.string()) {
                // ok.
            } else {
                self._check_class_exists(arg)?;
            }
        }
        Ok(())
    }

    /// Register constant of a class object
    /// Return class_ty
    // TODO: why not create the constant on class definition?
    fn _create_class_const(&mut self, name: &ConstName) -> TermTy {
        let ty = if name.args.is_empty() {
            name.to_ty(&[]).meta_ty()
        } else {
            // If the const is `A<B>`, also create its type `Meta:A<B>`
            self._create_specialized_meta_class(name)
        };
        let idx = self.register_string_literal(&name.string());
        let expr = Hir::class_literal(ty.clone(), name, idx);
        self.register_const_full(name.to_const_fullname(), expr);
        ty
    }

    /// Create `Meta:A<B>` when there is a const `A<B>`
    /// Return class_ty
    fn _create_specialized_meta_class(&mut self, name: &ConstName) -> TermTy {
        let mut typarams = &vec![];
        if let Some(c) = self.class_ctx() { typarams = &c.typarams }
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
        let tyargs = name.args.iter().map(|arg| arg.to_ty(typarams)).collect::<Vec<_>>();
        let cls = self
            .class_dict
            .find_class(&class_fullname(
                "Meta:".to_string() + &name.names.join("::"),
            ))
            .unwrap()
            .specialized_meta(&tyargs);
        let ty = cls.instance_ty.clone();
        self.class_dict.add_class(cls);
        ty
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
        self.convert_array_literal_(item_exprs)
    }

    fn convert_array_literal_(
        &mut self,
        item_exprs: Vec<HirExpression>,
    ) -> Result<HirExpression, Error> {
        // TODO #102: Support empty array literal
        let mut item_ty = if item_exprs.is_empty() {
            ty::raw("Object")
        } else {
            item_exprs[0].ty.clone()
        };

        for expr in &item_exprs {
            item_ty = self.nearest_common_ancestor_type(&item_ty, &expr.ty)
        }
        let ary_ty = ty::spe("Array", vec![item_ty]);

        Ok(Hir::array_literal(item_exprs, ary_ty))
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

fn lambda_ty(params: &[MethodParam], ret_ty: &TermTy) -> TermTy {
    let mut tyargs = params.iter().map(|x| x.ty.clone()).collect::<Vec<_>>();
    tyargs.push(ret_ty.clone());
    ty::spe(&format!("Fn{}", params.len()), tyargs)
}
