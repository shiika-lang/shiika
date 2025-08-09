use crate::mir;
use crate::mir::rewriter::MirRewriter;
use crate::mir::visitor::MirVisitor;
use crate::names::FunctionName;
use anyhow::Result;
use skc_hir::SkTypes;
use std::collections::{HashMap, HashSet, VecDeque};

/// Set Function.is_async (and asyncness in sk_types) to true or false.
///
/// This judgement is conservative because it is (possible, but) hard to
/// tell if a function is async or not when we support first-class functions.
/// It is safe to be false-positive (performance penalty aside).
pub fn run(mut mir: mir::Program, sk_types: &mut SkTypes) -> mir::Program {
    // Externs are known to be async or not
    let mut known = HashMap::new();
    for e in &mir.externs {
        known.insert(e.name.clone(), e.is_async());
    }
    for f in &mut mir.funcs {
        // User main needs to be async
        if f.name == mir::main_function_name() {
            f.asyncness = mir::Asyncness::Async;
        }
        match f.asyncness {
            mir::Asyncness::Async => {
                known.insert(f.name.clone(), true);
            }
            mir::Asyncness::Sync => {
                known.insert(f.name.clone(), false);
            }
            _ => {}
        }
    }
    let funcs: HashMap<_, _> = mir.funcs.iter().map(|f| (f.name.clone(), f)).collect();

    let mut q = VecDeque::from(funcs.values().map(|f| f.name.clone()).collect::<Vec<_>>());
    let mut unresolved_deps = HashMap::new();
    while let Some(name) = q.pop_front() {
        if known.contains_key(&name) || unresolved_deps.contains_key(&name) {
            continue;
        }
        Check::run(
            &funcs,
            &name,
            &mut known,
            &mut HashSet::new(),
            &mut unresolved_deps,
        );
    }
    // These functions did not have explicit async call so it is safe to mark as sync.
    for k in unresolved_deps.keys() {
        known.insert(k.clone(), false);
    }

    // Apply the result
    let mut u = Update { known: &known };
    u.set_func_asyncness(&mut mir);
    let new_mir = u.walk_mir(mir).unwrap();

    // Write back asyncness to SkTypes
    // REFACTOR: better data structure which do not require this
    for (name, is_async) in &known {
        let Some(m) = name.method_name() else {
            continue;
        };
        if let Some(sk_type) = sk_types.types.get_mut(&m.type_name) {
            if let Some((sig, _)) = sk_type.base_mut().method_sigs.get_mut(&m.first_name) {
                sig.asyncness = if *is_async {
                    skc_hir::Asyncness::Async
                } else {
                    skc_hir::Asyncness::Sync
                };
            }
        }
    }

    // Consistency check
    for f in &new_mir.funcs {
        let mut a = Assert::new();
        debug_assert!(a.check_func(f));
    }

    new_mir
}

/// Check if a function is async or not.
struct Check<'a> {
    is_async: bool,
    funcs: &'a HashMap<FunctionName, &'a mir::Function>,
    current_func: &'a FunctionName,
    known: &'a mut HashMap<FunctionName, bool>,
    checking: &'a mut HashSet<FunctionName>,
    depends: HashSet<FunctionName>,
    unresolved_deps: &'a mut HashMap<FunctionName, HashSet<FunctionName>>,
}
impl<'a> Check<'a> {
    fn run(
        funcs: &HashMap<FunctionName, &mir::Function>,
        fname: &FunctionName,
        known: &mut HashMap<FunctionName, bool>,
        checking: &mut HashSet<FunctionName>,
        unresolved_deps: &mut HashMap<FunctionName, HashSet<FunctionName>>,
    ) {
        let mut c = Check {
            is_async: false,
            funcs,
            current_func: fname,
            known,
            checking,
            depends: HashSet::new(),
            unresolved_deps,
        };
        c.checking.insert(fname.clone());
        let func = funcs.get(fname).unwrap();
        c.walk_expr(&func.body_stmts).unwrap();
        if c.depends.is_empty() {
            c.known.insert(fname.clone(), c.is_async);
        } else {
            c.unresolved_deps.insert(fname.clone(), c.depends);
        }
    }

    /// Called via MirVisitor
    fn visit_fexpr(&mut self, fexpr: &mir::TypedExpr) {
        match fexpr {
            (mir::Expr::FuncRef(ref name), _) => {
                if !self.known.contains_key(name) {
                    if name == self.current_func {
                        // Ignore call to itself
                    } else if self.checking.contains(name) {
                        // Avoid infinite recursion
                        self.depends.insert(name.clone());
                    } else {
                        // Check the function now
                        Check::run(
                            &self.funcs,
                            name,
                            &mut self.known,
                            &mut self.checking,
                            &mut self.unresolved_deps,
                        );
                    }
                }
                match self.known.get(name) {
                    Some(true) => {
                        // Calling an async function
                        self.is_async = true;
                    }
                    Some(false) => {
                        // Calling a non-async function.
                        // Let's see the rest
                    }
                    None => {
                        // The asyncness of the dependency is also unknown.
                        // Continue processing
                    }
                }
            }
            _ => {
                // Indirect calls
                self.is_async = match fexpr.1.as_fun_ty().asyncness {
                    mir::Asyncness::Async => true,
                    mir::Asyncness::Sync => false,
                    mir::Asyncness::Unknown => {
                        // Conservatively assume it is async
                        true
                    }
                    _ => unreachable!(),
                };
            }
        }
    }
}
impl<'a> MirVisitor for Check<'a> {
    fn visit_expr(&mut self, texpr: &mir::TypedExpr) -> Result<()> {
        // Short circuit
        if self.is_async {
            return Ok(());
        }
        match texpr {
            (mir::Expr::FunCall(fexpr, _), _) => {
                self.visit_fexpr(fexpr);
            }
            _ => {}
        }
        Ok(())
    }
}

/// Update function references to reflect the asyncness check result.
struct Update<'a> {
    known: &'a HashMap<FunctionName, bool>,
}
impl<'a> Update<'a> {
    fn set_func_asyncness(&self, mir: &mut mir::Program) {
        for f in &mut mir.funcs {
            let is_async = self.known.get(&f.name).unwrap();
            f.asyncness = (*is_async).into();
        }
    }
}
impl MirRewriter for Update<'_> {
    fn rewrite_expr(&mut self, texpr: mir::TypedExpr) -> Result<mir::TypedExpr> {
        match texpr.0 {
            // Apply known asyncness
            mir::Expr::FuncRef(ref name) => {
                let Some(is_async) = self.known.get(name) else {
                    panic!("Function {} is not found in known", name);
                };
                let mut fun_ty = texpr.1.into_fun_ty();
                fun_ty.asyncness = (*is_async).into();
                Ok((texpr.0, fun_ty.into()))
            }
            // Fix indirect calls
            mir::Expr::FunCall(mut fexpr, args) => {
                let mut fun_ty = fexpr.1.clone().into_fun_ty();
                if fun_ty.asyncness == mir::Asyncness::Unknown {
                    // Conservatively assume it is async
                    fun_ty.asyncness = mir::Asyncness::Async;
                }
                fexpr.1 = fun_ty.into();
                Ok(mir::Expr::fun_call(*fexpr, args))
            }
            _ => Ok(texpr),
        }
    }
}

/// Consistency check
struct Assert {
    found_async_call: bool,
}
impl Assert {
    fn new() -> Self {
        Assert {
            found_async_call: false,
        }
    }

    fn check_func(&mut self, f: &mir::Function) -> bool {
        self.walk_expr(&f.body_stmts).unwrap();
        if self.found_async_call && !f.asyncness.is_async() {
            panic!(
                "Function {} is marked as sync, but found async call",
                f.name
            );
        }
        true
    }

    fn check_funcall(&mut self, fexpr: &mir::TypedExpr) {
        self.found_async_call |= fexpr.1.as_fun_ty().asyncness.is_async();
    }
}
impl MirVisitor for Assert {
    fn visit_expr(&mut self, texpr: &mir::TypedExpr) -> Result<()> {
        match texpr {
            (mir::Expr::FunCall(fexpr, _), _) => {
                self.check_funcall(fexpr);
            }
            _ => {}
        }
        Ok(())
    }
}
