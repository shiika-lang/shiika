//! Lower `CreateTypeObject` to a call to `Meta:Class#_new`.
//!
//! Special case: `CreateTypeObject(Metaclass)` is left as-is because its
//! `.class` field must point to itself, which requires codegen-level handling.
use crate::mir;
use crate::mir::rewriter::MirRewriter;
use crate::names::FunctionName;
use anyhow::Result;
use shiika_core::names::ConstFullname;
use shiika_core::ty::{Erasure, TermTy};

pub fn run(mir: mir::Program) -> mir::Program {
    let funcs = mir.funcs.into_iter().map(compile_func).collect();
    mir::Program::new(mir.classes, mir.externs, funcs, mir.constants)
}

fn compile_func(orig_func: mir::Function) -> mir::Function {
    let new_body_stmts = Lower.walk_expr(orig_func.body_stmts).unwrap();
    mir::Function {
        body_stmts: new_body_stmts,
        ..orig_func
    }
}

struct Lower;

impl MirRewriter for Lower {
    fn rewrite_expr(&mut self, texpr: mir::TypedExpr) -> Result<mir::TypedExpr> {
        let new_texpr = match texpr.0 {
            mir::Expr::CreateTypeObject(ref ty) if ty.fullname.0 == "Metaclass" => {
                let result_ty = texpr.1.clone();
                self.lower_create_type_object_metaclass(result_ty)
            }
            mir::Expr::CreateTypeObject(ref ty) => {
                let ty = ty.clone();
                let result_ty = texpr.1.clone();
                self.lower_create_type_object(ty, result_ty)
            }
            _ => texpr,
        };
        Ok(new_texpr)
    }
}

impl Lower {
    /// Lower `CreateTypeObject(Metaclass)` into a `Meta:Class#_new` call followed by
    /// a `SetClassObj` to achieve `Metaclass.class == Metaclass`.
    fn lower_create_type_object_metaclass(&self, result_ty: mir::Ty) -> mir::TypedExpr {
        let create_call = call_meta_class_new(
            "Metaclass",
            null_as(mir::Ty::raw("Metaclass")), // will be set to itself by SetClassObj
        );
        let lvar_name = "__metaclass";
        let lvar_ty = mir::Ty::raw("Class");
        mir::Expr::exprs(vec![
            mir::Expr::lvar_decl(lvar_name, create_call, false),
            // Set Metaclass.class to itself
            mir::Expr::set_class_obj(
                mir::Expr::lvar_ref(lvar_name, lvar_ty.clone()),
                mir::Expr::lvar_ref(lvar_name, lvar_ty.clone()),
            ),
            mir::Expr::cast(
                mir::CastType::Force(result_ty),
                mir::Expr::lvar_ref(lvar_name, lvar_ty),
            ),
        ])
    }

    /// Lower `CreateTypeObject(ty)` into a call to `Meta:Class#_new`.
    fn lower_create_type_object(&self, ty: TermTy, result_ty: mir::Ty) -> mir::TypedExpr {
        let metaclass_obj = self.build_metaclass_obj(&ty);
        self.build_meta_class_new(&ty, metaclass_obj, result_ty)
    }

    /// Build the metaclass type object (eg: `Meta:Foo`) via `Metaclass#_new`.
    fn build_metaclass_obj(&self, ty: &TermTy) -> mir::TypedExpr {
        let meta_ty = ty.meta_ty();
        let meta_name = meta_ty.fullname.0.clone();

        let fun_ty = mir::FunTy::new(
            mir::Asyncness::Sync,
            vec![
                mir::Ty::raw("Metaclass"), // receiver
                mir::Ty::raw("String"),    // name
                mir::Ty::Ptr,              // vtable
                mir::Ty::Ptr,              // wtable
                mir::Ty::raw("Metaclass"), // meta_cls
                mir::Ty::raw("Class"),     // erasure_cls
            ],
            mir::Ty::raw("Metaclass"),
        );

        let args = vec![
            null_as(mir::Ty::raw("Metaclass")),
            mir::Expr::string_literal(meta_name),
            mir::Expr::class_vtable(Erasure::the_metaclass()),
            mir::Expr::null_ptr(),
            metaclass_constant(),
            null_as(mir::Ty::raw("Class")),
        ];

        mir::Expr::fun_call(
            mir::Expr::func_ref(FunctionName::method("Metaclass", "_new"), fun_ty),
            args,
        )
    }

    /// Build the main type object via `Meta:Class#_new`.
    fn build_meta_class_new(
        &self,
        ty: &TermTy,
        metaclass_obj: mir::TypedExpr,
        result_ty: mir::Ty,
    ) -> mir::TypedExpr {
        let call = call_meta_class_new(&ty.fullname.0, metaclass_obj);
        // Cast the result from `Class` to the expected meta type (e.g. `Meta:Foo`)
        mir::Expr::cast(mir::CastType::Force(result_ty), call)
    }
}

/// Call `Meta:Class#_new` with the given class name and meta_cls argument.
fn call_meta_class_new(name: &str, meta_cls: mir::TypedExpr) -> mir::TypedExpr {
    let fun_ty = mir::FunTy::new(
        mir::Asyncness::Sync,
        vec![
            mir::Ty::meta("Class"),    // receiver (Meta:Class)
            mir::Ty::raw("String"),    // name
            mir::Ty::Ptr,              // vtable
            mir::Ty::Ptr,              // wtable
            mir::Ty::raw("Metaclass"), // meta_cls
            mir::Ty::Ptr,              // erasure_cls
        ],
        mir::Ty::raw("Class"),
    );
    let args = vec![
        null_as(mir::Ty::meta("Class")),
        mir::Expr::string_literal(name),
        mir::Expr::class_vtable(Erasure::nonmeta("Class")),
        mir::Expr::null_ptr(),
        meta_cls,
        mir::Expr::null_ptr(),
    ];
    mir::Expr::fun_call(
        mir::Expr::func_ref(FunctionName::method("Meta:Class", "_new"), fun_ty),
        args,
    )
}

/// Null pointer cast to `ty`.
fn null_as(ty: mir::Ty) -> mir::TypedExpr {
    mir::Expr::cast(mir::CastType::Force(ty), mir::Expr::null_ptr())
}

/// Reference to the `Metaclass` constant, typed as `Metaclass`.
fn metaclass_constant() -> mir::TypedExpr {
    mir::Expr::const_ref(
        ConstFullname::toplevel("Metaclass"),
        mir::Ty::raw("Metaclass"),
    )
}
