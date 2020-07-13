# Parser

Directory: `src/parser/`

## Overview

- Hand-written parser (i.e. no parser generator)
- Stateful lexer

## Lexer

File: `src/parser/lexer.rs`

Lexer has a state (`LexerState`). Main purpose of this is to decide operators like `-` or `+` is whether unary or binary.

```
/// - `p(-x)`  # unary minus             ExprBegin
/// - `p(- x)` # unary minus             ExprBegin   
/// - `p( - x)`# unary minus             ExprBegin   
/// - `p- x`   # binary minus (unusual)  ExprEnd
/// - `p-x`    # binary minus            ExprEnd
/// - `p - x`  # binary minus            ExprArg
/// - `p -x`   # unary minus             ExprArg
/// - `1 -2`   # binary minus (unusual)  ExprArg  
```

This state is set by the parser.
