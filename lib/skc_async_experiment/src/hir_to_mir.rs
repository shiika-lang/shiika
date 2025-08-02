mod collect_allocs;
mod constants;
use crate::names::FunctionName;
use crate::{build, hir, mir};
use anyhow::Result;
use shiika_core::names::MethodFirstname;
use shiika_core::ty::TermTy;
use skc_hir::{MethodParam, MethodSignature, SkTypes};

pub fn run(
    hir: hir::CompilationUnit,
    target: &build::CompileTarget,
) -> Result<mir::CompilationUnit> {
    log::debug!("Building VTables");
    let vtables = skc_mir::VTables::build(&hir.sk_types, &hir.imports);
    let c = HirToMir {
        vtables: &vtables,
        imported_vtables: &hir.imports.vtables,
    };

    log::debug!("Converting user functions");
    let classes = c.convert_classes(&hir);
    let mut externs = convert_externs(&hir.imports.sk_types);
    for method_name in &hir.sk_types.rustlib_methods {
        externs.push(build_extern(&hir.sk_types.get_sig(method_name).unwrap()));
    }
    if let build::CompileTargetDetail::Bin { total_deps, .. } = &target.detail {
        externs.extend(constants::const_init_externs(total_deps));
    }

    // Constants
    let const_list = hir
        .program
        .constants
        .iter()
        .map(|(name, rhs)| (name.clone(), rhs.1.clone()))
        .collect::<Vec<_>>();
    let mut funcs: Vec<_> = hir
        .program
        .methods
        .into_iter()
        .map(|method| c.convert_method(method))
        .collect();
    funcs.push(constants::create_const_init_func(
        hir.package_name.as_ref(),
        hir.program
            .constants
            .into_iter()
            .map(|(name, rhs)| (name, c.convert_texpr(rhs)))
            .collect(),
    ));

    log::debug!("Converting top exprs");
    if let build::CompileTargetDetail::Bin { total_deps, .. } = &target.detail {
        funcs.push(c.create_user_main(hir.program.top_exprs, total_deps));
    } else {
        if hir.program.top_exprs.len() > 0 {
            panic!("Top level expressions are not allowed in library");
        }
    }
    let program = mir::Program::new(classes, externs, funcs, const_list);
    Ok(mir::CompilationUnit {
        program,
        sk_types: hir.sk_types,
        vtables,
        imported_constants: hir.imports.constants.into_iter().collect(),
        imported_vtables: hir.imports.vtables,
    })
}

struct HirToMir<'a> {
    vtables: &'a skc_mir::VTables,
    imported_vtables: &'a skc_mir::VTables,
}

impl<'a> HirToMir<'a> {
    fn convert_classes(&self, hir: &hir::CompilationUnit) -> Vec<mir::MirClass> {
        let mut v: Vec<_> = hir
            .sk_types
            .sk_classes()
            .map(|sk_class| {
                let ivars = sk_class
                    .ivars_ordered()
                    .iter()
                    .map(|ivar| (ivar.name.clone(), convert_ty(ivar.ty.clone())))
                    .collect();
                mir::MirClass {
                    name: sk_class.fullname().0.clone(),
                    ivars,
                }
            })
            .collect();
        for c in hir.imports.sk_types.sk_classes() {
            let ivars = c
                .ivars_ordered()
                .iter()
                .map(|ivar| (ivar.name.clone(), convert_ty(ivar.ty.clone())))
                .collect();
            v.push(mir::MirClass {
                name: c.fullname().0.clone(),
                ivars,
            });
        }
        v
    }

    fn convert_method(&self, method: hir::Method<TermTy>) -> mir::Function {
        let mut params = method
            .sig
            .params
            .clone()
            .into_iter()
            .map(|x| self.convert_param(x))
            .collect::<Vec<_>>();
        params.insert(
            0,
            mir::Param {
                ty: convert_ty(method.self_ty()),
                name: "self".to_string(),
            },
        );
        let name = method.name();
        let allocs = collect_allocs::run(&method.body_stmts);
        let body_stmts = self.insert_allocs(allocs, self.convert_texpr(method.body_stmts));
        mir::Function {
            asyncness: method.sig.asyncness.clone().into(),
            name,
            params,
            ret_ty: convert_ty(method.sig.ret_ty.clone()),
            body_stmts,
            sig: Some(method.sig),
        }
    }

    fn insert_allocs(
        &self,
        allocs: Vec<(String, TermTy)>,
        stmts: mir::TypedExpr,
    ) -> mir::TypedExpr {
        let mut stmts_vec = mir::expr::into_exprs(stmts);
        let mut new_stmts = vec![];
        for (name, ty) in allocs {
            new_stmts.push(mir::Expr::alloc(name, convert_ty(ty)));
        }
        new_stmts.extend(stmts_vec.drain(..));
        mir::Expr::exprs(new_stmts)
    }

    fn convert_param(&self, param: MethodParam) -> mir::Param {
        mir::Param {
            ty: convert_ty(param.ty),
            name: param.name,
        }
    }

    fn convert_texpr(&self, texpr: hir::TypedExpr<TermTy>) -> mir::TypedExpr {
        (self.convert_expr(texpr.0), convert_ty(texpr.1))
    }

    fn convert_texpr_vec(&self, exprs: Vec<hir::TypedExpr<TermTy>>) -> Vec<mir::TypedExpr> {
        exprs.into_iter().map(|x| self.convert_texpr(x)).collect()
    }

    fn convert_expr(&self, expr: hir::Expr<TermTy>) -> mir::Expr {
        match expr {
            hir::Expr::Number(i) => mir::Expr::Number(i),
            hir::Expr::StringLiteral(s) => call_string_new(s),
            hir::Expr::PseudoVar(p) => mir::Expr::PseudoVar(p),
            hir::Expr::LVarRef(s) => mir::Expr::LVarRef(s),
            hir::Expr::ArgRef(i, s) => {
                // +1 for the receiver
                mir::Expr::ArgRef(i + 1, s)
            }
            hir::Expr::ConstRef(resolved_const_name) => {
                mir::Expr::ConstRef(mir::mir_const_name(resolved_const_name))
            }
            hir::Expr::FuncRef(n) => mir::Expr::FuncRef(n),
            hir::Expr::FunCall(f, a) => {
                mir::Expr::FunCall(Box::new(self.convert_texpr(*f)), self.convert_texpr_vec(a))
            }
            hir::Expr::UnresolvedMethodCall(_, _, _) => {
                unreachable!("UnresolvedMethodCall should be resolved before this")
            }
            hir::Expr::ResolvedMethodCall(call_type, receiver, sig, args) => {
                let receiver_ty = receiver.1.clone();
                let mir_receiver = self.convert_texpr(*receiver);
                let func_ref = match call_type {
                    hir::expr::MethodCallType::Direct => method_func_ref(sig),
                    hir::expr::MethodCallType::Virtual => {
                        let method_idx = self
                            .lookup_vtable(&receiver_ty, &sig.fullname.first_name)
                            .unwrap_or_else(|| panic!("Method not found in vtable: {}", sig));

                        mir::Expr::vtable_ref(
                            mir_receiver.clone(),
                            method_idx,
                            sig.fullname.first_name.0.clone(),
                            build_fun_ty(&sig),
                        )
                    }
                    _ => todo!(),
                };
                let mut mir_args = self.convert_texpr_vec(args);
                mir_args.insert(0, mir_receiver);
                mir::Expr::FunCall(Box::new(func_ref), mir_args)
            }
            hir::Expr::If(c, t, e) => mir::Expr::If(
                Box::new(self.convert_texpr(*c)),
                Box::new(self.convert_texpr(*t)),
                Box::new(self.convert_texpr(*e)),
            ),
            hir::Expr::While(c, b) => mir::Expr::While(
                Box::new(self.convert_texpr(*c)),
                Box::new(self.convert_texpr(*b)),
            ),
            hir::Expr::Spawn(b) => mir::Expr::Spawn(Box::new(self.convert_texpr(*b))),
            hir::Expr::LVarDecl(s, rhs) => mir::Expr::Assign(s, Box::new(self.convert_texpr(*rhs))),
            hir::Expr::Assign(s, v) => mir::Expr::Assign(s, Box::new(self.convert_texpr(*v))),
            hir::Expr::ConstSet(name, v) => {
                mir::Expr::ConstSet(mir::mir_const_name(name), Box::new(self.convert_texpr(*v)))
            }
            hir::Expr::Return(v) => mir::Expr::Return(Box::new(self.convert_texpr(*v))),
            hir::Expr::Exprs(b) => mir::Expr::Exprs(self.convert_texpr_vec(b)),
            hir::Expr::Upcast(v, t) => mir::Expr::Cast(
                mir::CastType::Upcast(convert_ty(t)),
                Box::new(self.convert_texpr(*v)),
            ),
            hir::Expr::CreateObject(class_name) => mir::Expr::CreateObject(class_name.0),
            hir::Expr::CreateTypeObject(type_name) => mir::Expr::CreateTypeObject(type_name.0),
        }
    }

    fn lookup_vtable(&self, ty: &TermTy, method_name: &MethodFirstname) -> Option<usize> {
        self.vtables
            .find(ty, method_name)
            .or_else(|| self.imported_vtables.find(ty, method_name))
    }

    fn create_user_main(
        &self,
        top_exprs: Vec<hir::TypedExpr<TermTy>>,
        total_deps: &[String],
    ) -> mir::Function {
        let mut body_stmts = vec![];
        body_stmts.extend(constants::call_all_const_inits(total_deps));
        body_stmts.extend(top_exprs.into_iter().map(|expr| self.convert_texpr(expr)));
        body_stmts.push(mir::Expr::return_(mir::Expr::number(0)));
        mir::Function {
            asyncness: mir::Asyncness::Unknown,
            name: mir::main_function_name(),
            params: vec![],
            ret_ty: mir::Ty::Raw("Int".to_string()),
            body_stmts: mir::Expr::exprs(body_stmts),
            sig: None,
        }
    }
}

fn convert_ty(ty: TermTy) -> mir::Ty {
    match ty.fn_x_info() {
        Some(tys) => {
            let mut param_tys = tys
                .into_iter()
                .map(|x| convert_ty(x.clone()))
                .collect::<Vec<_>>();
            let ret_ty = param_tys.pop().unwrap();
            mir::Ty::Fun(mir::FunTy {
                asyncness: mir::Asyncness::Unknown,
                param_tys,
                ret_ty: Box::new(ret_ty),
            })
        }
        None => match &ty.fullname.0[..] {
            "Shiika::Internal::Ptr" => mir::Ty::Ptr,
            "Shiika::Internal::Int64" => mir::Ty::Int64,
            _ => mir::Ty::Raw(ty.fullname.0),
        },
    }
}

fn convert_externs(imports: &SkTypes) -> Vec<mir::Extern> {
    imports
        .types
        .values()
        .flat_map(|sk_type| {
            sk_type
                .base()
                .method_sigs
                .unordered_iter()
                .map(|(sig, _)| build_extern(sig))
        })
        .collect()
}

fn build_extern(sig: &MethodSignature) -> mir::Extern {
    mir::Extern {
        name: FunctionName::unmangled(&sig.fullname.full_name),
        fun_ty: build_fun_ty(sig),
    }
}

fn build_fun_ty(sig: &MethodSignature) -> mir::FunTy {
    let mut param_tys = sig
        .params
        .iter()
        .map(|x| convert_ty(x.ty.clone()))
        .collect::<Vec<_>>();
    param_tys.insert(0, convert_ty(sig.fullname.type_name.to_ty()));
    mir::FunTy::new(
        sig.asyncness.clone().into(),
        param_tys,
        convert_ty(sig.ret_ty.clone()),
    )
}

fn call_string_new(s: String) -> mir::Expr {
    let string_new = mir::Expr::func_ref(
        FunctionName::unmangled("Meta:String#new"),
        mir::FunTy {
            asyncness: mir::Asyncness::Unknown,
            param_tys: vec![mir::Ty::raw("Meta:String"), mir::Ty::Ptr, mir::Ty::Int64],
            ret_ty: Box::new(mir::Ty::raw("String")),
        },
    );
    let bytesize = s.len() as i64;
    mir::Expr::fun_call(
        string_new,
        vec![
            mir::Expr::const_ref("::String", mir::Ty::raw("Meta:String")),
            mir::Expr::string_ref(s),
            mir::Expr::raw_i64(bytesize),
        ],
    )
    .0
}

fn method_func_ref(sig: MethodSignature) -> mir::TypedExpr {
    let fname = FunctionName::unmangled(&sig.fullname.full_name);
    let fun_ty = build_fun_ty(&sig);
    mir::Expr::func_ref(fname, fun_ty)
}
