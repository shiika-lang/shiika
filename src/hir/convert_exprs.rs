use crate::ast::*;
use crate::error;
use crate::error::Error;
use crate::hir::hir_maker::HirMaker;
use crate::hir::hir_maker_context::*;
use crate::hir::*;
use crate::parser::token::Token;
use crate::type_checking;

/// Result of looking up a lvar
#[derive(Debug)]
enum LVarInfo {
    /// Found in the current scope
    CurrentScope { ty: TermTy, name: String },
    /// Found in the current method/lambda argument
    Argument { ty: TermTy, idx: usize },
    /// Found in outer scope
    OuterScope {
        ty: TermTy,
        /// Index of the lvar in `captures`
        cidx: usize,
        readonly: bool,
    },
}

impl LVarInfo {
    /// The type of the lvar
    fn ty(&self) -> &TermTy {
        match self {
            LVarInfo::CurrentScope { ty, .. } => &ty,
            LVarInfo::Argument { ty, .. } => &ty,
            LVarInfo::OuterScope { ty, .. } => &ty,
        }
    }

    /// Returns HirExpression to refer this lvar
    fn ref_expr(&self) -> HirExpression {
        match self {
            LVarInfo::CurrentScope { ty, name } => Hir::lvar_ref(ty.clone(), name.clone()),
            LVarInfo::Argument { ty, idx } => Hir::arg_ref(ty.clone(), *idx),
            LVarInfo::OuterScope {
                ty, cidx, readonly, ..
            } => Hir::lambda_capture_ref(ty.clone(), *cidx, *readonly),
        }
    }

    /// Returns HirExpression to update this lvar
    fn assign_expr(&self, expr: HirExpression) -> HirExpression {
        match self {
            LVarInfo::CurrentScope { name, .. } => Hir::lvar_assign(name, expr),
            LVarInfo::Argument { .. } => panic!("[BUG] Cannot reassign argument"),
            LVarInfo::OuterScope { cidx, .. } => Hir::lambda_capture_write(*cidx, expr),
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

        let if_ty = if then_hirs.ty.is_never_type() {
            else_hirs.ty.clone()
        } else if then_hirs.ty.is_void_type() {
            ty::raw("Void")
        } else {
            then_hirs.ty.clone()
        };

        Ok(Hir::if_expression(if_ty, cond_hir, then_hirs, else_hirs))
    }

    fn convert_while_expr(
        &mut self,
        cond_expr: &AstExpression,
        body_exprs: &[AstExpression],
    ) -> Result<HirExpression, Error> {
        let cond_hir = self.convert_expr(cond_expr)?;
        type_checking::check_condition_ty(&cond_hir.ty, "while")?;

        let mut current = CtxKind::While;
        self.ctx.swap_current(&mut current);
        let body_hirs = self.convert_exprs(body_exprs)?;
        self.ctx.swap_current(&mut current);

        Ok(Hir::while_expression(cond_hir, body_hirs))
    }

    fn convert_break_expr(&mut self) -> Result<HirExpression, Error> {
        let from;
        if self.ctx.current == CtxKind::Lambda {
            let lambda_ctx = self.ctx.lambda_mut();
            if lambda_ctx.is_fn {
                return Err(error::program_error("`break' inside a fn"));
            } else {
                // OK for now. This `break` still may be invalid
                // (eg. `ary.map{ break }`) but it cannot be checked here
                lambda_ctx.has_break = true;
                from = HirBreakFrom::Block;
            }
        } else if self.ctx.current == CtxKind::While {
            from = HirBreakFrom::While;
        } else {
            return Err(error::program_error("`break' outside a loop"));
        }
        Ok(Hir::break_expression(from))
    }

    fn convert_return_expr(&mut self, arg: &Option<Box<AstExpression>>) -> Result<HirExpression, Error> {
        let from = self._validate_return()?;
        let arg_expr = if let Some(x) = arg {
            self.convert_expr(x)?
        } else {
            void_const_ref()
        };
        self._validate_return_type(&arg_expr.ty)?;
        Ok(Hir::return_expression(from, arg_expr))
    }

    /// Check if `return' is valid in the current context
    fn _validate_return(&self) -> Result<HirReturnFrom, Error> {
        if let Some(lambda_ctx) = self.ctx.lambdas.last() {
            if lambda_ctx.is_fn {
                Ok(HirReturnFrom::Fn)
            } else if self.ctx.method.is_some() {
                Ok(HirReturnFrom::Block)
            } else {
                Err(error::program_error("`return' outside a loop"))
            }
        } else if self.ctx.method.is_some() {
            Ok(HirReturnFrom::Method)
        } else {
            Err(error::program_error("`return' outside a loop"))
        }
    }

    /// Check if the argument of `return' is valid
    fn _validate_return_type(&self, arg_ty: &TermTy) -> Result<(), Error> {
        if self.ctx.current_is_fn() {
            // TODO
        } else if let Some(method_ctx) = &self.ctx.method {
            type_checking::check_return_arg_type(&self.class_dict, arg_ty, &method_ctx.signature)?;
        }
        Ok(())
    }

    fn convert_lvar_assign(
        &mut self,
        name: &str,
        rhs: &AstExpression,
        is_var: &bool,
    ) -> Result<HirExpression, Error> {
        let expr = self.convert_expr(rhs)?;
        // For `var x`, `x` should not be exist
        if *is_var && self._lookup_var(name).is_some() {
            return Err(error::program_error(&format!(
                "variable `{}' already exists",
                name
            )));
        }
        if let Some(lvar_info) = self._find_var(name, true)? {
            // Reassigning
            type_checking::check_reassign_var(&lvar_info.ty(), &expr.ty, name)?;
            Ok(lvar_info.assign_expr(expr))
        } else {
            // Create new lvar
            self.ctx.declare_lvar(name, expr.ty.clone(), !is_var);
            Ok(Hir::lvar_assign(name, expr))
        }
    }

    fn convert_ivar_assign(
        &mut self,
        name: &str,
        rhs: &AstExpression,
        is_var: &bool,
    ) -> Result<HirExpression, Error> {
        let expr = self.convert_expr(rhs)?;
        let self_ty = self.ctx.self_ty();

        if self.ctx.in_initializer() {
            let idx = self.declare_ivar(name, &expr.ty, !is_var)?;
            return Ok(Hir::ivar_assign(name, idx, expr, *is_var, self_ty.clone()));
        }

        if let Some(ivar) = self.class_dict.find_ivar(&self_ty.fullname, name) {
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
            Ok(Hir::ivar_assign(name, ivar.idx, expr, false, self_ty))
        } else {
            Err(error::program_error(&format!(
                "instance variable `{}' not found",
                name
            )))
        }
    }

    /// Declare a new ivar
    fn declare_ivar(&mut self, name: &str, ty: &TermTy, readonly: bool) -> Result<usize, Error> {
        let self_ty = &self.ctx.self_ty();
        let method_ctx = self.ctx.method.as_mut().unwrap();
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
        type_args: &[ConstName],
    ) -> Result<HirExpression, Error> {
        let arg_hirs = arg_exprs
            .iter()
            .map(|arg_expr| self.convert_expr(arg_expr))
            .collect::<Result<Vec<_>, _>>()?;

        // Check if this is a lambda invocation
        if receiver_expr.is_none() {
            if let Some(lvar) = self._lookup_var(&method_name.0) {
                if let Some(ret_ty) = lvar.ty().fn_x_info() {
                    return Ok(Hir::lambda_invocation(ret_ty, lvar.ref_expr(), arg_hirs));
                }
            }
        }

        let receiver_hir = match receiver_expr {
            Some(expr) => self.convert_expr(&expr)?,
            // Implicit self
            _ => self.convert_self_expr()?,
        };
        let mut method_tyargs = vec![];
        for const_name in type_args {
            method_tyargs.push(self._resolve_method_tyarg(const_name)?);
        }
        self._make_method_call(receiver_hir, &method_name, arg_hirs, &method_tyargs)
    }

    /// Resolve a method tyarg (a ConstName) into a TermTy
    /// eg.
    ///     ary.map<Array<T>>(f)
    ///             ~~~~~~~~
    ///             => TermTy(Array<TyParamRef(T)>)
    fn _resolve_method_tyarg(&self, name: &ConstName) -> Result<TermTy, Error> {
        let class_typarams = self.current_class_typarams();
        let method_typarams = self.current_method_typarams();
        let ret = name.to_ty(&class_typarams, &method_typarams);
        Ok(ret)
    }

    /// Resolve the method and create HirMethodCall
    fn _make_method_call(
        &self,
        receiver_hir: HirExpression,
        method_name: &MethodFirstname,
        mut arg_hirs: Vec<HirExpression>,
        method_tyargs: &[TermTy],
    ) -> Result<HirExpression, Error> {
        let specialized = receiver_hir.ty.is_specialized();
        let class_fullname = &receiver_hir.ty.fullname;
        let (sig, found_class_name) =
            self.class_dict
                .lookup_method(&receiver_hir.ty, method_name, method_tyargs)?;

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
        is_fn: &bool,
    ) -> Result<HirExpression, Error> {
        self.lambda_ct += 1;
        let lambda_id = self.lambda_ct;
        let hir_params = signature::convert_params(
            params,
            &self.current_class_typarams(),
            &self.current_method_typarams(),
        );

        // Convert lambda body
        self.ctx
            .lambdas
            .push(LambdaCtx::new(*is_fn, hir_params.clone()));

        let mut current = CtxKind::Lambda;
        self.ctx.swap_current(&mut current);
        let hir_exprs = self.convert_exprs(exprs)?;
        self.ctx.swap_current(&mut current);

        let mut lambda_ctx = self.ctx.lambdas.pop().unwrap();
        Ok(Hir::lambda_expr(
            lambda_ty(&hir_params, &hir_exprs.ty), // ty
            format!("lambda_{}", lambda_id),       // name
            hir_params,
            hir_exprs,
            self._resolve_lambda_captures(lambda_ctx.captures), // hir_captures
            extract_lvars(&mut lambda_ctx.lvars),               // lvars
            lambda_ctx.has_break,
        ))
    }

    /// Resolve LambdaCapture into HirExpression
    /// Also, concat lambda_captures to outer_captures
    fn _resolve_lambda_captures(
        &mut self,
        lambda_captures: Vec<LambdaCapture>,
    ) -> Vec<HirLambdaCapture> {
        let mut ret = vec![];
        for cap in lambda_captures {
            let captured_here = match self.ctx.current {
                CtxKind::Lambda => cap.ctx_depth == (self.ctx.lambdas.len() as isize) - 1,
                _ => cap.ctx_depth == -1,
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
                let cidx = self.ctx.push_lambda_capture(cap);
                ret.push(HirLambdaCapture::CaptureFwd { cidx, ty });
            }
        }
        ret
    }

    /// Generate local variable reference or method call with implicit receiver(self)
    fn convert_bare_name(&mut self, name: &str) -> Result<HirExpression, Error> {
        if let Some(lvar_info) = self._find_var(name, false)? {
            Ok(lvar_info.ref_expr())
        } else {
            Err(error::program_error(&format!(
                "variable `{}' was not found",
                name
            )))
        }
    }

    /// Return the variable of the given name, if any
    fn _lookup_var(&mut self, name: &str) -> Option<LVarInfo> {
        self._find_var(name, false).unwrap()
    }

    /// Find the variable of the given name.
    /// If it is a free variable, ctx.captures will be modified
    fn _find_var(&mut self, name: &str, updating: bool) -> Result<Option<LVarInfo>, Error> {
        let cidx = if self.ctx.current == CtxKind::Lambda {
            self.ctx.lambdas.last().unwrap().captures.len()
        } else {
            0 // this value is never used
        };
        let mut first = true;
        let mut caps = vec![];
        let mut ret = None;
        for (lvars, params, depth) in self.ctx.lvar_scopes() {
            // Local
            if let Some(lvar) = lvars.get(name) {
                if updating && lvar.readonly {
                    return Err(error::program_error(&format!(
                        "cannot reassign to `{}' (Hint: declare it with `var')",
                        name
                    )));
                }
                if first {
                    ret = Some(LVarInfo::CurrentScope {
                        ty: lvar.ty.clone(),
                        name: name.to_string(),
                    });
                    break;
                } else {
                    // !first == there are more than one scope == we're in a lambda
                    // and capturing outer lvar
                    caps.push(LambdaCapture {
                        ctx_depth: depth,
                        ty: lvar.ty.clone(),
                        detail: LambdaCaptureDetail::CapLVar {
                            name: name.to_string(),
                        },
                    });
                    ret = Some(LVarInfo::OuterScope {
                        ty: lvar.ty.clone(),
                        cidx,
                        readonly: false,
                    });
                    break;
                }
            }
            // Arg
            if let Some((idx, param)) = signature::find_param(&params, name) {
                if updating {
                    return Err(error::program_error(&format!(
                        "you cannot reassign to argument `{}'",
                        name
                    )));
                }
                if first {
                    return Ok(Some(LVarInfo::Argument {
                        ty: param.ty.clone(),
                        idx,
                    }));
                } else {
                    // !first == there are more than one scope == we're in a lambda
                    // and capturing the outer arg
                    caps.push(LambdaCapture {
                        ctx_depth: depth,
                        ty: param.ty.clone(),
                        detail: LambdaCaptureDetail::CapFnArg { idx },
                    });
                    ret = Some(LVarInfo::OuterScope {
                        ty: param.ty.clone(),
                        cidx,
                        readonly: true,
                    });
                    break;
                }
            }
            first = false;
        }
        if !caps.is_empty() {
            caps.into_iter().for_each(|cap| {
                self.ctx.push_lambda_capture(cap);
            });
        }
        Ok(ret)
    }

    fn convert_ivar_ref(&self, name: &str) -> Result<HirExpression, Error> {
        let self_ty = self.ctx.self_ty();
        let found = self
            .class_dict
            .find_ivar(&self_ty.fullname, name)
            .or_else(|| self.ctx.method.as_ref().unwrap().iivars.get(name));
        match found {
            Some(ivar) => Ok(Hir::ivar_ref(
                ivar.ty.clone(),
                name.to_string(),
                ivar.idx,
                self_ty,
            )),
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
            return Some((found, fullname));
        }

        let fullname = name.under_namespace(&self.ctx.namespace());
        self.constants.get(&fullname).map(|found| (found, fullname))
    }

    /// Check `name` refers proper class name
    /// eg. For `A<B<C>>`, check A, B and C exists
    fn _check_class_exists(&self, name: &ConstName) -> Result<(), Error> {
        if !self.class_dict.class_exists(&name.names.join("::")) {
            return Err(error::program_error(&format!(
                "constant `{:?}' was not found",
                name.names.join("::")
            )));
        }
        if name.args.is_empty() {
            return Ok(());
        }
        let mut typarams = self.current_class_typarams();
        typarams.append(&mut self.current_method_typarams());
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
            name.to_ty(&[], &[]).meta_ty()
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
        let class_typarams = self.current_class_typarams();
        let method_typarams = self.current_method_typarams();
        let tyargs = name
            .args
            .iter()
            .map(|arg| arg.to_ty(&class_typarams, &method_typarams))
            .collect::<Vec<_>>();
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
        Ok(Hir::self_expression(self.ctx.self_ty()))
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

/// Check if `break` in block is valid
fn check_break_in_block(sig: &MethodSignature, last_arg: &mut HirExpression) -> Result<(), Error> {
    if let HirExpressionBase::HirLambdaExpr { has_break, .. } = last_arg.node {
        if has_break {
            if sig.ret_ty == ty::raw("Void") {
                match &mut last_arg.node {
                    HirExpressionBase::HirLambdaExpr { ret_ty, .. } => {
                        std::mem::swap(ret_ty, &mut ty::raw("Void"));
                    }
                    _ => return Err(error::bug("unexpected type")),
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
