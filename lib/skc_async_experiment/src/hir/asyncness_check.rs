use crate::hir;
use crate::hir::rewriter::HirRewriter;
use crate::hir::visitor::HirVisitor;
use anyhow::Result;
use std::collections::{HashMap, HashSet, VecDeque};

/// Set Function.is_async to true or false.
/// This judgement is conservative because it is (possible, but) hard to
/// tell if a function is async or not when we support first-class functions.
/// It is safe to be false-positive (performance penalty aside).
pub fn run(mut hir: hir::Program) -> hir::Program {
    // Externs are known to be async or not
    let mut known = HashMap::new();
    for e in &hir.externs {
        known.insert(e.name.clone(), e.is_async);
    }
    for f in &hir.funcs {
        match f.asyncness {
            hir::Asyncness::Async => {
                known.insert(f.name.clone(), true);
            }
            hir::Asyncness::Sync => {
                known.insert(f.name.clone(), false);
            }
            _ => {}
        }
    }
    let funcs: HashMap<_, _> = hir.funcs.iter().map(|f| (f.name.clone(), f)).collect();

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
    u.set_func_asyncness(&mut hir);
    let new_hir = u.walk_hir(hir).unwrap();

    // Consistency check
    for f in &new_hir.funcs {
        let mut a = Assert::new();
        debug_assert!(a.check_func(f));
    }

    new_hir
}

/// Check if a function is async or not.
struct Check<'a> {
    is_async: bool,
    funcs: &'a HashMap<String, &'a hir::Function>,
    current_func: &'a str,
    known: &'a mut HashMap<String, bool>,
    checking: &'a mut HashSet<String>,
    depends: HashSet<String>,
    unresolved_deps: &'a mut HashMap<String, HashSet<String>>,
}
impl<'a> Check<'a> {
    fn run(
        funcs: &HashMap<String, &hir::Function>,
        fname: &str,
        known: &mut HashMap<String, bool>,
        checking: &mut HashSet<String>,
        unresolved_deps: &mut HashMap<String, HashSet<String>>,
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
        c.checking.insert(fname.to_string());
        let func = funcs.get(fname).unwrap();
        c.walk_exprs(&func.body_stmts).unwrap();
        if c.depends.is_empty() {
            let mut is_async = c.is_async;
            // HACK: force endif-functions to be marked as async
            if fname.ends_with("'e") {
                is_async = true;
            }
            c.known.insert(fname.to_string(), is_async);
        } else {
            c.unresolved_deps.insert(fname.to_string(), c.depends);
        }
    }

    /// Called via HirVisitor
    fn visit_fexpr(&mut self, fexpr: &hir::TypedExpr) {
        match fexpr {
            (hir::Expr::FuncRef(ref name), _) => {
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
                    hir::Asyncness::Async => true,
                    hir::Asyncness::Sync => false,
                    hir::Asyncness::Unknown => {
                        // Conservatively assume it is async
                        true
                    }
                    _ => unreachable!(),
                };
            }
        }
    }
}
impl<'a> HirVisitor for Check<'a> {
    fn visit_expr(&mut self, texpr: &hir::TypedExpr) -> Result<()> {
        // Short circuit
        if self.is_async {
            return Ok(());
        }
        match texpr {
            (hir::Expr::FunCall(fexpr, _), _) => {
                self.visit_fexpr(fexpr);
            }
            _ => {}
        }
        Ok(())
    }
}

/// Update function references to reflect the asyncness check result.
struct Update<'a> {
    known: &'a HashMap<String, bool>,
}
impl<'a> Update<'a> {
    fn set_func_asyncness(&self, hir: &mut hir::Program) {
        for f in &mut hir.funcs {
            let is_async = self.known.get(&f.name).unwrap();
            f.asyncness = (*is_async).into();
        }
    }
}
impl HirRewriter for Update<'_> {
    fn rewrite_expr(&mut self, texpr: hir::TypedExpr) -> Result<hir::TypedExpr> {
        match texpr.0 {
            // Apply known asyncness
            hir::Expr::FuncRef(ref name) => {
                let Some(is_async) = self.known.get(name) else {
                    panic!("Function {} is not found in known", name);
                };
                let mut fun_ty = texpr.1.into_fun_ty();
                fun_ty.asyncness = (*is_async).into();
                Ok((texpr.0, fun_ty.into()))
            }
            // Fix indirect calls
            hir::Expr::FunCall(mut fexpr, args) => {
                let mut fun_ty = fexpr.1.clone().into_fun_ty();
                if fun_ty.asyncness == hir::Asyncness::Unknown {
                    // Conservatively assume it is async
                    fun_ty.asyncness = hir::Asyncness::Async;
                }
                fexpr.1 = fun_ty.into();
                Ok(hir::Expr::fun_call(*fexpr, args))
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

    fn check_func(&mut self, f: &hir::Function) -> bool {
        self.walk_exprs(&f.body_stmts).unwrap();
        if self.found_async_call && !f.asyncness.is_async() {
            panic!(
                "Function {} is marked as sync, but found async call",
                f.name
            );
        }
        true
    }

    fn check_funcall(&mut self, fexpr: &hir::TypedExpr) {
        self.found_async_call |= fexpr.1.as_fun_ty().asyncness.is_async();
    }
}
impl HirVisitor for Assert {
    fn visit_expr(&mut self, texpr: &hir::TypedExpr) -> Result<()> {
        match texpr {
            (hir::Expr::FunCall(fexpr, _), _) => {
                self.check_funcall(fexpr);
            }
            _ => {}
        }
        Ok(())
    }
}
