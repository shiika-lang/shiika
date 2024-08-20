use crate::ast::{self, Spanned};
use anyhow::{anyhow, Result};
//use ariadne::{Label, Report, ReportKind, Source};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_till},
    character::complete::{alpha1, alphanumeric1, multispace0, multispace1},
    combinator::{eof, opt, recognize},
    multi::{many0, many0_count, separated_list0},
    number,
    sequence::{delimited, pair, preceded, separated_pair, terminated},
    IResult,
};
use nom_locate::{self, position};
type Span<'a> = nom_locate::LocatedSpan<&'a str>;
type E<'a> = nom::error::VerboseError<Span<'a>>;

//fn render_parse_error(src: &str, e: E) -> String {
//    let mut rendered = vec![];
//    Report::build(ReportKind::Error, "", span.start)
//        .with_message(msg.clone())
//        .with_label(Label::new(("", span)).with_message(msg))
//        .finish()
//        .write(("", Source::from(src)), &mut rendered)
//        .unwrap();
//    String::from_utf8_lossy(&rendered).to_string()
//}

pub fn parse(src: &str) -> Result<ast::Program> {
    let input = Span::new(src);
    match parse_program(input) {
        Ok((_, prog)) => Ok(prog),
        Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
            // https://github.com/fflorent/nom_locate/issues/36#issuecomment-1013469728
            let errors = e
                .errors
                .into_iter()
                .map(|(input, error)| (*input.fragment(), error))
                .collect();
            let s = nom::error::convert_error(src, nom::error::VerboseError { errors });
            Err(anyhow!("{}", s))
        }
        _ => unreachable!(),
    }
}

fn parse_program<'a>(s: Span<'a>) -> IResult<Span, ast::Program<'a>, E> {
    let (s, pos) = position(s)?;
    let (s, decls) = parse_decls(s)?;
    Ok((s, (decls, pos)))
}

fn parse_decls(s: Span) -> IResult<Span, Vec<ast::Declaration>, E> {
    let (s, decls) = many0(delimited(parse_comments, parse_decl, parse_comments))(s)?;
    let (s, _) = multispace0(s)?;
    let (s, _) = eof(s)?;
    Ok((s, decls))
}

fn parse_comments(s: Span) -> IResult<Span, (), E> {
    let (s, _) = multispace0(s)?;
    let (s, _) = many0(delimited(multispace0, parse_comment, multispace0))(s)?;
    Ok((s, ()))
}

fn parse_comment(s: Span) -> IResult<Span, (), E> {
    let (s, _) = tag("#")(s)?;
    let (s, _) = take_till(|c| c == '\n')(s)?;
    Ok((s, ()))
}

fn parse_decl<'a>(s: Span<'a>) -> IResult<Span<'a>, ast::Declaration<'a>, E> {
    alt((parse_extern, parse_function))(s)
}

fn parse_extern<'a>(s: Span<'a>) -> IResult<Span<'a>, ast::Declaration<'a>, E> {
    let (s, _) = tag("extern")(s)?;
    let (s, opts) = opt(parse_extern_flags)(s)?;
    let (s, _) = multispace1(s)?;
    let (s, (name, pos)) = parse_ident(s)?;
    let (s, params) = parse_param_list(s)?;
    let (s, _) = delimited(multispace0, tag("->"), multispace0)(s)?;
    let (s, (ret_ty, _)) = parse_ty(s)?;
    let (s, _) = preceded(multispace0, tag(";"))(s)?;

    let mut is_async = false;
    let mut is_internal = false;
    for flag in opts.unwrap_or_default() {
        match &flag.0[..] {
            "async" => is_async = true,
            "internal" => is_internal = true,
            _ => panic!("unknown extern flag: {:?}", flag),
        }
    }

    let e = ast::Extern {
        is_async,
        is_internal,
        name,
        params,
        ret_ty,
    };
    Ok((s, ast::Declaration::Extern((e, pos))))
}

fn parse_extern_flags<'a>(s: Span<'a>) -> IResult<Span<'a>, Vec<Spanned<String>>, E> {
    delimited(tag("("), separated_list0(tag(","), parse_ident), tag(")"))(s)
}

fn parse_param_list<'a>(s: Span<'a>) -> IResult<Span<'a>, Vec<ast::Param>, E> {
    delimited(
        tag("("),
        delimited(multispace0, parse_params, multispace0),
        tag(")"),
    )(s)
}

fn parse_params<'a>(s: Span<'a>) -> IResult<Span<'a>, Vec<ast::Param>, E> {
    separated_list0(delimited(multispace0, tag(", "), multispace0), parse_param)(s)
}

fn parse_param<'a>(s: Span<'a>) -> IResult<Span<'a>, ast::Param, E> {
    let (s, (ty, _)) = parse_ty(s)?;
    let (s, _) = multispace1(s)?;
    let (s, (name, _)) = parse_ident(s)?;
    Ok((s, ast::Param { ty, name }))
}

fn parse_function<'a>(s: Span<'a>) -> IResult<Span<'a>, ast::Declaration<'a>, E> {
    let (s, _) = tag("fun")(s)?;
    let (s, _) = multispace1(s)?;
    let (s, (name, pos)) = parse_ident(s)?;
    let (s, params) = parse_param_list(s)?;
    let (s, _) = delimited(multispace0, tag("->"), multispace0)(s)?;
    let (s, (ret_ty, _)) = parse_ty(s)?;
    let (s, body_stmts) = preceded(multispace0, parse_block)(s)?;
    let e = ast::Function {
        name,
        params,
        ret_ty,
        body_stmts,
    };
    Ok((s, ast::Declaration::Function((e, pos))))
}

fn parse_block<'a>(s: Span<'a>) -> IResult<Span<'a>, Vec<ast::SpannedExpr<'a>>, E> {
    delimited(
        tag("{"),
        many0(delimited(parse_comments, parse_stmt, parse_comments)),
        tag("}"),
    )(s)
}

/// An expr terminated with ';'
fn parse_stmt<'a>(s: Span<'a>) -> IResult<Span<'a>, ast::SpannedExpr<'a>, E> {
    terminated(
        alt((
            parse_assign,
            alt((
                parse_alloc,
                alt((
                    parse_return,
                    alt((parse_if, alt((parse_while, parse_expr)))),
                )),
            )),
        )),
        terminated(multispace0, tag(";")),
    )(s)
}

fn parse_assign<'a>(s: Span<'a>) -> IResult<Span<'a>, ast::SpannedExpr<'a>, E> {
    let (s, (name, pos)) = parse_ident(s)?;
    let (s, _) = delimited(multispace0, tag("="), multispace0)(s)?;
    let (s, rhs) = preceded(multispace0, parse_expr)(s)?;
    Ok((s, (ast::Expr::Assign(name.to_string(), Box::new(rhs)), pos)))
}

fn parse_alloc<'a>(s: Span<'a>) -> IResult<Span<'a>, ast::SpannedExpr<'a>, E> {
    let (s, pos) = position(s)?;
    let (s, _) = tag("alloc")(s)?;
    let (s, _) = multispace1(s)?;
    let (s, (name, _pos)) = parse_ident(s)?;
    Ok((s, (ast::Expr::Alloc(name.to_string()), pos)))
}

fn parse_return<'a>(s: Span<'a>) -> IResult<Span<'a>, ast::SpannedExpr<'a>, E> {
    let (s, pos) = position(s)?;
    let (s, _) = tag("return")(s)?;
    let (s, _) = multispace1(s)?;
    let (s, expr) = parse_expr(s)?;
    Ok((s, (ast::Expr::Return(Box::new(expr)), pos)))
}

fn parse_if<'a>(s: Span<'a>) -> IResult<Span<'a>, ast::SpannedExpr<'a>, E> {
    let (s, pos) = position(s)?;
    let (s, _) = tag("if")(s)?;
    let (s, cond) = delimited(
        multispace0,
        delimited(
            tag("("),
            delimited(multispace0, parse_expr, multispace0),
            tag(")"),
        ),
        multispace0,
    )(s)?;
    let (s, then) = parse_block(s)?;
    let (s, _) = multispace0(s)?;
    let (s, els) = opt(parse_else)(s)?;
    Ok((s, (ast::Expr::If(Box::new(cond), then, els), pos)))
}

fn parse_while<'a>(s: Span<'a>) -> IResult<Span<'a>, ast::SpannedExpr<'a>, E> {
    let (s, pos) = position(s)?;
    let (s, _) = tag("while")(s)?;
    let (s, cond) = delimited(
        multispace0,
        delimited(
            tag("("),
            delimited(multispace0, parse_expr, multispace0),
            tag(")"),
        ),
        multispace0,
    )(s)?;
    let (s, exprs) = parse_block(s)?;
    Ok((s, (ast::Expr::While(Box::new(cond), exprs), pos)))
}

fn parse_else<'a>(s: Span<'a>) -> IResult<Span<'a>, Vec<ast::SpannedExpr<'a>>, E> {
    let (s, _) = tag("else")(s)?;
    let (s, _) = multispace0(s)?;
    parse_block(s)
}

fn parse_expr<'a>(s: Span<'a>) -> IResult<Span<'a>, ast::SpannedExpr<'a>, E> {
    let (s, left) = parse_additive(s)?;
    let (s, _) = multispace0(s)?;
    let (s, chain) = many0(separated_pair(
        alt((
            tag("=="),
            alt((
                tag("!="),
                alt((tag("<"), alt((tag("<="), alt((tag(">"), tag(">="))))))),
            )),
        )),
        multispace0,
        parse_additive,
    ))(s)?;
    Ok((s, build_op_calls(left, chain)))
}

fn parse_additive<'a>(s: Span<'a>) -> IResult<Span<'a>, ast::SpannedExpr<'a>, E> {
    let (s, left) = parse_multiplicative(s)?;
    let (s, _) = multispace0(s)?;
    let (s, chain) = many0(separated_pair(
        alt((tag("+"), tag("-"))),
        multispace0,
        parse_multiplicative,
    ))(s)?;
    Ok((s, build_op_calls(left, chain)))
}

fn parse_multiplicative<'a>(s: Span<'a>) -> IResult<Span<'a>, ast::SpannedExpr<'a>, E> {
    let (s, left) = parse_atomic(s)?;
    let (s, _) = multispace0(s)?;
    let (s, chain) = many0(separated_pair(
        alt((tag("*"), tag("/"))),
        multispace0,
        parse_atomic,
    ))(s)?;
    Ok((s, build_op_calls(left, chain)))
}

fn build_op_calls<'a>(
    expr: ast::SpannedExpr<'a>,
    chain: Vec<(Span, ast::SpannedExpr<'a>)>,
) -> ast::SpannedExpr<'a> {
    chain.into_iter().fold(expr, |acc, (op, right)| {
        let pos = acc.1.clone();
        (
            ast::Expr::OpCall(op.to_string(), Box::new(acc), Box::new(right)),
            pos,
        )
    })
}

fn parse_atomic<'a>(s: Span<'a>) -> IResult<Span<'a>, ast::SpannedExpr<'a>, E> {
    alt((
        parse_para,
        alt((parse_funcall, alt((parse_number, parse_varref)))),
    ))(s)
}

fn parse_para<'a>(s: Span<'a>) -> IResult<Span<'a>, ast::SpannedExpr<'a>, E> {
    let (s, pos) = position(s)?;
    let (s, _) = tag("para")(s)?;
    let (s, _) = multispace0(s)?;
    let (s, exprs) = parse_block(s)?;
    Ok((s, (ast::Expr::Para(exprs), pos)))
}

fn parse_funcall<'a>(s: Span<'a>) -> IResult<Span<'a>, ast::SpannedExpr<'a>, E> {
    let (s, (f, pos)) = parse_varref(s)?;
    let (s, args) = parse_arg_list(s)?;
    let fexpr = (f, pos.clone());
    Ok((s, (ast::Expr::FunCall(Box::new(fexpr), args), pos)))
}

fn parse_arg_list<'a>(s: Span<'a>) -> IResult<Span<'a>, Vec<ast::SpannedExpr<'a>>, E> {
    delimited(
        tag("("),
        delimited(multispace0, parse_args, multispace0),
        tag(")"),
    )(s)
}

fn parse_args<'a>(s: Span<'a>) -> IResult<Span<'a>, Vec<ast::SpannedExpr<'a>>, E> {
    separated_list0(delimited(multispace0, tag(","), multispace0), parse_expr)(s)
}

fn parse_number<'a>(s: Span<'a>) -> IResult<Span<'a>, ast::SpannedExpr<'a>, E> {
    // TODO: Just parse integer
    let (s, pos) = position(s)?;
    let (s, n) = number::complete::double(s)?;
    Ok((s, (ast::Expr::Number(n.floor() as i64), pos)))
}

fn parse_varref<'a>(s: Span<'a>) -> IResult<Span<'a>, ast::SpannedExpr<'a>, E> {
    let (s, (name, pos)) = parse_ident(s)?;
    Ok((s, (ast::Expr::VarRef(name), pos)))
}

fn parse_ident<'a>(s: Span<'a>) -> IResult<Span<'a>, Spanned<String>, E> {
    let (s, pos) = position(s)?;
    let (s, name) = recognize(pair(
        alt((alpha1, tag("_"))),
        many0_count(alt((alphanumeric1, tag("_")))),
    ))(s)?;
    Ok((s, (name.to_string(), pos)))
}

fn parse_ty<'a>(s: Span<'a>) -> IResult<Span<'a>, Spanned<ast::Ty>, E> {
    alt((parse_ty_fun, parse_ty_raw))(s)
}

fn parse_ty_fun<'a>(s: Span<'a>) -> IResult<Span<'a>, Spanned<ast::Ty>, E> {
    let (s, pos) = position(s)?;
    let (s, _) = tag("FN(")(s)?;
    let (s, param_tys) = delimited(tag("("), separated_list0(tag(","), parse_ty), tag(")"))(s)?;
    let (s, _) = tag("->")(s)?;
    let (s, (ret_ty, _)) = parse_ty(s)?;
    let (s, _) = tag(")")(s)?;
    let fun_ty = ast::FunTy {
        param_tys: param_tys.into_iter().map(|(ty, _)| ty).collect(),
        ret_ty: Box::new(ret_ty),
    };
    Ok((s, (ast::Ty::Fun(fun_ty), pos)))
}

fn parse_ty_raw<'a>(s: Span<'a>) -> IResult<Span<'a>, Spanned<ast::Ty>, E> {
    let (s, pos) = position(s)?;
    let (s, name) = alphanumeric1(s)?;
    Ok((s, (ast::Ty::Raw(name.to_string()), pos)))
}
