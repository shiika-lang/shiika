mod collect_allocs;
mod constants;
use crate::names::FunctionName;
use crate::{build, hir, mir};
use anyhow::Result;
use shiika_core::ty::TermTy;
use skc_hir::SkTypes;
use std::collections::HashSet;

pub fn run(
    hir: hir::CompilationUnit,
    target: &build::CompileTarget,
) -> Result<mir::CompilationUnit> {
    log::debug!("Start");
    let vtables = skc_mir::VTables::build(&hir.sk_types, &hir.imports);
    let c = HirToMir { vtables: &vtables };

    let classes = c.convert_classes(&hir);
    log::debug!("VTables built");
    let mut externs = c.convert_externs(&hir.imports.sk_types, hir.imported_asyncs);
    if let build::CompileTargetDetail::Bin { total_deps, .. } = &target.detail {
        externs.extend(constants::const_init_externs(total_deps));
    }
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
    log::debug!("User functions converted");
    funcs.push(constants::create_const_init_func(
        hir.package_name.as_ref(),
        hir.program
            .constants
            .into_iter()
            .map(|(name, rhs)| (name, c.convert_texpr(rhs)))
            .collect(),
    ));
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
        imported_types: hir.imports.sk_types,
    })
}

struct HirToMir<'a> {
    vtables: &'a skc_mir::VTables,
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
                    .map(|ivar| (ivar.name.clone(), self.convert_ty(ivar.ty.clone())))
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
                .map(|ivar| (ivar.name.clone(), self.convert_ty(ivar.ty.clone())))
                .collect();
            v.push(mir::MirClass {
                name: c.fullname().0.clone(),
                ivars,
            });
        }
        v
    }

    fn convert_externs(
        &self,
        imports: &SkTypes,
        imported_asyncs: Vec<FunctionName>,
    ) -> Vec<mir::Extern> {
        let asyncs: HashSet<FunctionName> = HashSet::from_iter(imported_asyncs);
        imports
            .0
            .values()
            .flat_map(|sk_type| {
                sk_type.base().method_sigs.unordered_iter().map(|(sig, _)| {
                    let fname = FunctionName::from_sig(sig);
                    let asyncness = if asyncs.contains(&fname) {
                        mir::Asyncness::Async
                    } else {
                        mir::Asyncness::Sync
                    };
                    let mut param_tys = sig
                        .params
                        .iter()
                        .map(|x| self.convert_ty(x.ty.clone()))
                        .collect::<Vec<_>>();
                    param_tys.insert(0, self.convert_ty(sk_type.term_ty()));
                    let fun_ty =
                        mir::FunTy::new(asyncness, param_tys, self.convert_ty(sig.ret_ty.clone()));
                    mir::Extern {
                        name: fname,
                        fun_ty,
                    }
                })
            })
            .collect()
    }

    fn convert_method(&self, method: hir::Method<TermTy>) -> mir::Function {
        let mut params = method
            .params
            .into_iter()
            .map(|x| self.convert_param(x))
            .collect::<Vec<_>>();
        params.insert(
            0,
            mir::Param {
                ty: self.convert_ty(method.self_ty),
                name: "self".to_string(),
            },
        );
        let allocs = collect_allocs::run(&method.body_stmts);
        let body_stmts = self.insert_allocs(allocs, self.convert_texpr(method.body_stmts));
        mir::Function {
            asyncness: mir::Asyncness::Unknown,
            name: method.name,
            params,
            ret_ty: self.convert_ty(method.ret_ty),
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
            new_stmts.push(mir::Expr::alloc(name, self.convert_ty(ty)));
        }
        new_stmts.extend(stmts_vec.drain(..));
        mir::Expr::exprs(new_stmts)
    }

    fn convert_ty(&self, ty: TermTy) -> mir::Ty {
        match ty.fn_x_info() {
            Some(tys) => {
                let mut param_tys = tys
                    .into_iter()
                    .map(|x| self.convert_ty(x.clone()))
                    .collect::<Vec<_>>();
                let ret_ty = param_tys.pop().unwrap();
                mir::Ty::Fun(mir::FunTy {
                    asyncness: mir::Asyncness::Unknown,
                    param_tys,
                    ret_ty: Box::new(ret_ty),
                })
            }
            _ => mir::Ty::Raw(ty.fullname.0),
        }
    }

    fn convert_param(&self, param: hir::Param) -> mir::Param {
        mir::Param {
            ty: self.convert_ty(param.ty),
            name: param.name,
        }
    }

    fn convert_texpr(&self, texpr: hir::TypedExpr<TermTy>) -> mir::TypedExpr {
        (
            self.convert_expr(texpr.0, &texpr.1),
            self.convert_ty(texpr.1),
        )
    }

    fn convert_texpr_vec(&self, exprs: Vec<hir::TypedExpr<TermTy>>) -> Vec<mir::TypedExpr> {
        exprs.into_iter().map(|x| self.convert_texpr(x)).collect()
    }

    fn convert_expr(&self, expr: hir::Expr<TermTy>, ty: &TermTy) -> mir::Expr {
        match expr {
            hir::Expr::Number(i) => mir::Expr::Number(i),
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
            hir::Expr::ResolvedMethodCall(call_type, receiver, method_name, args) => {
                let func_ref = match call_type {
                    hir::expr::MethodCallType::Direct => {
                        todo!();
                    }
                    hir::expr::MethodCallType::Virtual => {
                        let method_idx = self
                            .vtables
                            .method_idx(&receiver.1, &method_name)
                            .expect("Method not found in vtable")
                            .0;
                        mir::Expr::VTableRef(
                            Box::new(self.convert_texpr(*receiver)),
                            *method_idx,
                            method_name.0,
                        )
                    }
                    _ => todo!(),
                };
                let func_ty = mir::Ty::Fun(mir::FunTy {
                    asyncness: mir::Asyncness::Unknown,
                    param_tys: args.iter().map(|x| self.convert_ty(x.1.clone())).collect(),
                    ret_ty: Box::new(self.convert_ty(ty.clone())),
                });
                let func_ref_t = (func_ref, func_ty);
                mir::Expr::FunCall(Box::new(func_ref_t), self.convert_texpr_vec(args))
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
                mir::CastType::Upcast(self.convert_ty(t)),
                Box::new(self.convert_texpr(*v)),
            ),
            hir::Expr::CreateObject(class_name) => mir::Expr::CreateObject(class_name.0),
            hir::Expr::CreateTypeObject(type_name) => mir::Expr::CreateTypeObject(type_name.0),
        }
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
