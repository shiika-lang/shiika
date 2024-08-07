use crate::ast;

type PegError = peg::error::ParseError<peg::str::LineCol>;

pub fn parse(src: &str) -> Result<ast::Program, PegError> {
    milika_parser::program(&src)
}

peg::parser! {
  grammar milika_parser() for str {
    pub rule program() -> Vec<ast::Declaration>
      = _ l:(decl() ** _) _ { l }

    rule decl() -> ast::Declaration
      = e:extern() { ast::Declaration::Extern(e) }
      / f:func() { ast::Declaration::Function(f) }

    rule extern() -> ast::Extern
      = "extern" flags:extern_flags()? _ sig:sig() {
        let fs = flags.unwrap_or((false, false));
        ast::Extern {
          name: sig.0,
          params: sig.1,
          ret_ty: sig.2,
          is_async: fs.0,
          is_internal: fs.1
        }
      }

    rule extern_flags() -> (bool, bool)
      = "(async)" { (true, false) }
      / "(internal)" { (false, true) }

    rule func() -> ast::Function
      = "fun" _ sig:sig() _ "{" _ body:stmts() _ "}" {
        ast::Function {
          name: sig.0,
          params: sig.1,
          ret_ty: sig.2,
          body_stmts: body,
        }
      }

    rule sig() -> (String, Vec<ast::Param>, ast::Ty)
      = name:ident() _ "(" _ params:params() _ ")" _ "->" _ ret_ty:ty() {
        (name, params, ret_ty)
      }

    rule params() -> Vec<ast::Param>
      = (param() ** (_ "," _))

    rule param() -> ast::Param
      = param_ty:ty() _ name:ident() { ast::Param { ty: param_ty, name: name } }

    rule ty() -> ast::Ty
      = fun_ty()
      / name:ident() { ast::Ty::Raw(name) }

    rule fun_ty() -> ast::Ty
      = "FN" _ "(" _ "(" _ param_tys:tys() _ ")" _ "->" _ ret_ty:ty() _ ")" {
        ast::Ty::Fun(ast::FunTy { param_tys, ret_ty: Box::new(ret_ty) })
      }

    rule tys() -> Vec<ast::Ty>
      = (ty() ** (_ "," _))

    rule stmts() -> Vec<ast::Expr>
      = stmts:(expr() ** _) { stmts }

    rule expr() -> ast::Expr
      = alloc()
      / if()
      / yield()
      / while()
      / spawn()
      / return()
      / assign()
      / equality()

    rule alloc() -> ast::Expr
      = "alloc" _ name:ident() { ast::Expr::Alloc(name) }

    rule if() -> ast::Expr
      = "if" _ cond:expr() _ "{" _ then:stmts() _ "}" _ else_:else()? {
        ast::Expr::If(Box::new(cond), then, else_)
      }

    rule yield() -> ast::Expr
      = "yield" _ e:expr() { ast::Expr::Yield(Box::new(e)) }

    rule else() -> Vec<ast::Expr>
      = "else" _ "{" _ stmts:stmts() _ "}" { stmts }

    rule while() -> ast::Expr
      = "while" _ cond:expr() _ "{" _ stmts:stmts() _ "}" {
        ast::Expr::While(Box::new(cond), stmts)
      }

    rule spawn() -> ast::Expr
      = "spawn" _ func:expr() { ast::Expr::Spawn(Box::new(func)) }

    rule return() -> ast::Expr
      = "return" _ e:expr() { ast::Expr::Return(Box::new(e)) }

    rule args() -> Vec<ast::Expr>
      = (expr() ** (_ "," _))

    rule callee() -> ast::Expr
      = v:var_ref() { v }
      / "(" _ e:expr() _ ")" { e }

    rule assign() -> ast::Expr
      = v:ident() _ "=" _ e:expr() { ast::Expr::Assign(v, Box::new(e)) }

    rule equality() -> ast::Expr
      = l:additive() _ "==" _ r:additive() { ast::Expr::OpCall("==".to_string(), Box::new(l), Box::new(r)) }
      / l:additive() _ "!=" _ r:additive() { ast::Expr::OpCall("!=".to_string(), Box::new(l), Box::new(r)) }
      / l:additive() _ "<" _ r:additive() { ast::Expr::OpCall("<".to_string(), Box::new(l), Box::new(r)) }
      / l:additive() _ "<=" _ r:additive() { ast::Expr::OpCall("<=".to_string(), Box::new(l), Box::new(r)) }
      / l:additive() _ ">" _ r:additive() { ast::Expr::OpCall(">".to_string(), Box::new(l), Box::new(r)) }
      / l:additive() _ ">=" _ r:additive() { ast::Expr::OpCall(">=".to_string(), Box::new(l), Box::new(r)) }
      / additive()

    rule additive() -> ast::Expr
      = l:multiplicative() _ "+" _ r:additive() { ast::Expr::OpCall("+".to_string(), Box::new(l), Box::new(r)) }
      / l:multiplicative() _ "-" _ r:additive() { ast::Expr::OpCall("-".to_string(), Box::new(l), Box::new(r)) }
      / multiplicative()

    rule multiplicative() -> ast::Expr
      = l:atom() _ "*" _ r:multiplicative() { ast::Expr::OpCall("*".to_string(), Box::new(l), Box::new(r)) }
      / l:atom() _ "/" _ r:multiplicative() { ast::Expr::OpCall("/".to_string(), Box::new(l), Box::new(r)) }
      / atom()

    rule atom() -> ast::Expr
      = n:number() { n }
      / funcall()
      / v:var_ref() { v }
      / "(" _ e:expr() _ ")" { e }

    rule funcall() -> ast::Expr
      = f:callee() _ "(" _ args:args() _ ")" {
        ast::Expr::FunCall(Box::new(f), args)
      }

    rule number() -> ast::Expr
      = n:$(['0'..='9']+) {?
        n.parse()
          .or(Err("cannot parser number as u64"))
          .map(|n| ast::Expr::Number(n))
      }

    rule var_ref() -> ast::Expr
      = i:ident() { ast::Expr::VarRef(i) }

    rule ident() -> String
      = s:$(['_' | 'a'..='z' | 'A'..='Z']+) { s.to_string() }

    //rule comment() = [' ' | '\t']* "#" (!"\n" [_])*
    //rule comment_line() = comment() "\n"
    //rule white_line() = [' ' | '\t']* "\n"

    //rule atmosphere() = comment() "\n" / [' ' | '\t' | '\n']

    // Optional whitespace
    rule _ = (" " / "\t" / "\n" / ("#" (!"\n" [_])*))*
    // Mandatory whitespace
    //rule __ = [' ' | '\t']+
    // Mandatory newline
    //rule newline() = (([' ' | '\t']* ("#" (!"\n" [_])*)?) ** "\n") "\n"
    //rule newline() = _ "\n"
  }
}
