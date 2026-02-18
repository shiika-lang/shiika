# Lambda Captures Implementation Plan (Read + Write + Fn Object)

## Goal
Make this work:
```shiika
var y = 5                         # var (mutable)
let f = fn(x: Int){ y = y + x }   # Lambda stored in variable
f(10)
print y  // Expected: 15
```

## Current State
- Lambda without captures: Working
- `HirLambdaCaptureRef` / `HirLambdaCaptureWrite`: TODO in mirgen.rs

---

## Approach Overview

### 1. Cell for Captured `var` Variables
`var` variables that are captured (readonly or writable) use heap-allocated cells:
- Declaration: `var y = 5` → `var y = cell_new(5)`
- Read: `y` → `cell_get(y)`
- Write: `y = v` → `cell_set(y, v)`
- Capture: store cell pointer (shared reference)

**Semantics:**
- `let` + capture: value copy (let is immutable, so no sharing needed)
- `var` + capture: Cell shared (outer scope changes visible to lambda)
- Not captured: no Cell needed

### 2. Fn Base Class + Fn0-Fn9 Subclasses
Like old system, use Fn classes to represent closures:

```shiika
# packages/core/lib/fn.sk
base class Fn
  def initialize(@func: Shiika::Internal::Ptr, @captures: Shiika::Internal::Ptr)
  end
end

class Fn0<R> : Fn
end

class Fn1<A1, R> : Fn
end

# ... Fn2 through Fn9
```

### 3. Lambda Representation

| Component | Description |
|-----------|-------------|
| Lambda function | Generated as `lambda_N(fn_obj, arg1, ...)` |
| Fn object | Contains `@func` (pointer to lambda_N) and `@captures` (array) |
| Captures array | Contains values (readonly) or cell pointers (writable) |

---

## Data Flow Example

```
Source:
  var y = 5                              # var (mutable)
  let f = fn(x: Int){ y = y + x }
  f(10)
  print y

Transformed:
  var y = cell_new(5)                    # Allocate cell (because var + captured)
  let captures = [y]                      # Captures array (cell pointers)
  let f = Fn1.new(@lambda_0, captures)   # Create closure object
  invoke f with (10)                      # Call closure
  print cell_get(y)                       # Read through cell

lambda_0(fn_obj: Fn1, x: Int):
  let captures = fn_obj.@captures         # Get captures from Fn object
  let y_cell = captures[0]                # Get cell pointer
  let y_val = cell_get(y_cell)            # Read current value
  cell_set(y_cell, y_val + x)             # Write new value
```

---

## Implementation Steps

### (Done) Step 0.1: Codegen Prelude

**File:** `lib/skc_async_experiment/src/codegen/prelude.rs`

Add external declarations:

```rust
("shiika_cell_new", FunTy::sync(vec![Ty::Any], Ty::Ptr)),
("shiika_cell_get", FunTy::sync(vec![Ty::Ptr], Ty::Any)),
("shiika_cell_set", FunTy::sync(vec![Ty::Ptr, Ty::Any], Ty::CVoid)),
```

### (Done) Step 0.2: impl. mir::Expr::NativeArrayRef

Which takes an llvm array and index and returns the element.
This is reused for captures array access (no need for separate CapturesRef).

### (Done) Step 1: Define Fn Classes

**File:** `packages/core/lib/fn.sk` (new file)

```shiika
# Base class with common fields
base class Fn
  def initialize(@func: Shiika::Internal::Ptr, @captures: Shiika::Internal::Ptr)
  end
end

# Subclasses for different arities (type parameters for type safety)
class Fn0<R> : Fn
end

class Fn1<A1, R> : Fn
end

class Fn2<A1, A2, R> : Fn
end

class Fn3<A1, A2, A3, R> : Fn
end

class Fn4<A1, A2, A3, A4, R> : Fn
end

class Fn5<A1, A2, A3, A4, A5, R> : Fn
end

class Fn6<A1, A2, A3, A4, A5, A6, R> : Fn
end

class Fn7<A1, A2, A3, A4, A5, A6, A7, R> : Fn
end

class Fn8<A1, A2, A3, A4, A5, A6, A7, A8, R> : Fn
end

class Fn9<A1, A2, A3, A4, A5, A6, A7, A8, A9, R> : Fn
end
```

### (Done) Step 2: Add Cell Runtime Functions

**File:** `packages/core/ext/src/runtime/cell.rs` (new file)

```rust
use std::alloc::{alloc, Layout};

#[no_mangle]
pub extern "C" fn shiika_cell_new(value: u64) -> *mut u64 {
    unsafe {
        let layout = Layout::new::<u64>();
        let ptr = alloc(layout) as *mut u64;
        *ptr = value;
        ptr
    }
}

#[no_mangle]
pub extern "C" fn shiika_cell_get(cell: *mut u64) -> u64 {
    unsafe { *cell }
}

#[no_mangle]
pub extern "C" fn shiika_cell_set(cell: *mut u64, value: u64) {
    unsafe { *cell = value; }
}
```

### (Done) Step 3: Add MIR Expressions

**File:** `lib/skc_async_experiment/src/mir/expr.rs`

```rust
pub enum Expr {
    // ... existing ...

    /// Allocate a cell: shiika_cell_new(value) -> Ptr
    CellNew(Box<TypedExpr>),

    /// Read from cell: shiika_cell_get(cell) -> value
    CellGet(Box<TypedExpr>),

    /// Write to cell: shiika_cell_set(cell, value)
    CellSet { cell: Box<TypedExpr>, value: Box<TypedExpr> },

    // Note: Use existing NativeArrayRef for captures array access
}
```

### Step 4+5: Collect captured vars and transform LVar access (2-pass in mirgen)

**File:** `lib/skc_async_experiment/src/mirgen.rs`

HIRは上から順に生成されるため、`HirLVarDecl`の時点ではその変数がキャプチャされるか不明。
そのためHIR側の変更は行わず、mirgenで2-passにする:

**Pass 1: Collect cell vars**

関数/メソッドのHIR bodyをスキャンし、`HirLambdaExpr`のcapturesから
cellが必要な変数名の集合(`HashSet<String>`)を作る。
条件: `captured && !readonly`（`var`かつキャプチャされている）

```rust
/// Scan HIR expressions for HirLambdaExpr nodes and collect
/// lvar names that need Cell wrapping (captured + !readonly).
fn collect_cell_vars(exprs: &HirExpression) -> HashSet<String> {
    let mut cell_vars = HashSet::new();
    // Walk the HIR tree, find HirLambdaExpr nodes
    // For each capture where !readonly && detail is CaptureLVar:
    //   cell_vars.insert(name)
    cell_vars
}
```

**Pass 2: Generate MIR using cell_vars set**

```rust
// LVar read
HirExpressionBase::HirLVarRef { name } => {
    if cell_vars.contains(&name) {
        // Captured var: read through cell
        let cell = mir::Expr::lvar_ref(name, mir::Ty::Ptr);
        mir::Expr::cell_get(cell, convert_ty(expr.ty))
    } else {
        mir::Expr::lvar_ref(name, convert_ty(expr.ty))
    }
}

// LVar declaration (init)
HirExpressionBase::HirLVarDecl { name, rhs, readonly } => {
    let mir_rhs = self.convert_expr(*rhs);
    if cell_vars.contains(&name) {
        // var y = 5 → var y = cell_new(5)
        mir::Expr::LVarDecl(name, mir::Expr::cell_new(mir_rhs), true)
    } else {
        mir::Expr::LVarDecl(name, mir_rhs, !readonly)
    }
}

// LVar reassign
HirExpressionBase::HirLVarAssign { name, rhs } => {
    let mir_rhs = self.convert_expr(*rhs);
    if cell_vars.contains(&name) {
        // y = v → cell_set(y, v)
        let cell = mir::Expr::lvar_ref(name, mir::Ty::Ptr);
        mir::Expr::cell_set(cell, mir_rhs)
    } else {
        mir::Expr::lvar_set(name, mir_rhs)
    }
}
```

**No HIR changes needed** — `HirLVarRef`, `HirLVarAssign`, `HirLVarDecl` remain as-is.

### Step 6: Handle Lambda Creation (HirLambdaExpr)

**File:** `lib/skc_async_experiment/src/mirgen.rs`

```rust
HirExpressionBase::HirLambdaExpr {
    name, params, exprs, captures, lvars, ret_ty, has_break,
} => {
    if has_break {
        todo!("Lambda break not yet supported")
    }

    // 1. Generate the lambda function
    let lambda_func = self.create_lambda_function(
        &name, &params, &captures, &exprs, &lvars, &ret_ty
    );
    let func_name = lambda_func.name.clone();
    self.lambda_funcs.push(lambda_func);

    // 2. Create captures array
    let capture_values: Vec<mir::TypedExpr> = captures.iter()
        .map(|cap| self.get_capture_value(cap))
        .collect();
    let captures_array = mir::Expr::create_native_array(capture_values);

    // 3. Create Fn object: FnN.new(func_ptr, captures)
    let fn_class = format!("Fn{}", params.len());
    let func_ptr = mir::Expr::func_ptr(func_name);

    mir::Expr::create_fn_object(fn_class, func_ptr, captures_array)
}

// Helper to get capture value
fn get_capture_value(&self, cap: &HirLambdaCapture) -> mir::TypedExpr {
    match &cap.detail {
        HirLambdaCaptureDetail::CaptureLVar { name } => {
            if !cap.readonly {
                // var lvar: pass cell pointer (lvar holds cell)
                mir::Expr::lvar_ref(name.clone(), mir::Ty::Ptr)
            } else {
                // let lvar: pass value directly
                mir::Expr::lvar_ref(name.clone(), convert_ty(cap.ty.clone()))
            }
        }
        HirLambdaCaptureDetail::CaptureArg { idx } => {
            // Args are captured by value
            mir::Expr::arg_ref(idx + 1, "captured_arg", convert_ty(cap.ty.clone()))
        }
        _ => todo!("Unsupported capture type: {:?}", cap.detail),
    }
}
```

### Step 7: Handle Lambda Invocation (HirLambdaInvocation)

**File:** `lib/skc_async_experiment/src/mirgen.rs`

```rust
HirExpressionBase::HirLambdaInvocation { lambda_expr, arg_exprs } => {
    let fn_obj = self.convert_expr(*lambda_expr);

    // Extract @func from Fn object
    let func_ptr = mir::Expr::ivar_ref(fn_obj.clone(), 0, "@func", mir::Ty::Ptr);

    // Build args: [fn_obj, arg0, arg1, ...]
    let mut mir_args = vec![fn_obj];
    mir_args.extend(arg_exprs.into_iter().map(|arg| self.convert_expr(arg)));

    // Indirect function call through func_ptr
    mir::Expr::indirect_call(func_ptr, mir_args, convert_ty(expr.ty))
}
```

### Step 8: Handle Capture Access Inside Lambda

**File:** `lib/skc_async_experiment/src/mirgen.rs`

Lambda function signature: `lambda_N(fn_obj: FnN, arg1, arg2, ...)`

```rust
fn create_lambda_function(...) -> mir::Function {
    // First param is Fn object
    let fn_class = format!("Fn{}", params.len());
    let mut mir_params = vec![mir::Param::new(mir::Ty::raw(&fn_class), "$fn")];

    // Then explicit params
    mir_params.extend(params.iter().map(|p| mir::Param {
        ty: convert_ty(p.ty.clone()),
        name: p.name.clone(),
    }));

    // ... convert body with capture context ...
}
```

Handle `HirLambdaCaptureRef`:

Use `HirLambdaCapture.readonly` to determine if Cell is needed:

```rust
// HirLambdaCaptureRef has `readonly` field
HirExpressionBase::HirLambdaCaptureRef { idx, readonly } => {
    let fn_obj = mir::Expr::arg_ref(0, "$fn", mir::Ty::Ptr);
    let captures = mir::Expr::ivar_ref(fn_obj, 1, "@captures", mir::Ty::Ptr);

    if !readonly {
        // var capture: read through cell
        let cell = mir::Expr::native_array_ref(captures, idx, mir::Ty::Ptr);
        mir::Expr::cell_get(cell, convert_ty(expr.ty))
    } else {
        // let capture: direct value
        mir::Expr::native_array_ref(captures, idx, convert_ty(expr.ty))
    }
}
```

Handle `HirLambdaCaptureWrite`:

```rust
// HirLambdaCaptureWrite only occurs for var captures, always uses Cell
HirExpressionBase::HirLambdaCaptureWrite { cidx, rhs } => {
    let fn_obj = mir::Expr::arg_ref(0, "$fn", mir::Ty::Ptr);
    let captures = mir::Expr::ivar_ref(fn_obj, 1, "@captures", mir::Ty::Ptr);
    let cell = mir::Expr::native_array_ref(captures, cidx, mir::Ty::Ptr);
    let value = self.convert_expr(*rhs);
    mir::Expr::cell_set(cell, value)
}
```

---

## Summary of Changes

| File | Changes |
|------|---------|
| ~~`lib/skc_hir/src/lib.rs`~~ | ~~(不要: HIR変更なし)~~ |
| ~~`lib/skc_ast2hir/src/convert_exprs.rs`~~ | ~~(不要: HIR変更なし)~~ |
| `packages/core/lib/fn.sk` | Define Fn base class + Fn0-Fn9 subclasses |
| `lib/shiika_ffi/src/async_/cell.rs` | Cell runtime functions |
| `lib/skc_async_experiment/src/mir/expr.rs` | Add CellNew, CellGet, CellSet (use existing NativeArrayRef for captures) |
| `lib/skc_async_experiment/src/mirgen.rs` | 2-pass: (1) collect cell vars from captures (2) handle cells, Fn creation/invocation |
| `lib/skc_async_experiment/src/mir_lowering/` | Add resolve_cells.rs pass |
| `lib/skc_async_experiment/src/codegen/prelude.rs` | Add cell function externs |

---

## Ivar Indices for Fn Class

| Index | Name | Type | Description |
|-------|------|------|-------------|
| 0 | @func | Ptr | Pointer to lambda function |
| 1 | @captures | Ptr | Pointer to captures array |

Note: Unlike old system, we don't need `@the_self` or `@exit_status` for the MVP.

---

## Verification

```bash
rake async_test
```

Test cases:
1. **Readonly capture, immediate invoke:**
   ```shiika
   let y = 5
   (fn(x: Int){ print x + y })(10)  # prints 15
   ```

2. **Writable capture, immediate invoke:**
   ```shiika
   let y = 5
   (fn(x: Int){ y = y + x })(10)
   print y  # prints 15
   ```

3. **Lambda stored in variable:**
   ```shiika
   let y = 5
   let f = fn(x: Int){ y = y + x }
   f(10)
   print y  # prints 15
   ```

4. **Multiple captures:**
   ```shiika
   let a = 1
   let b = 2
   let f = fn(x: Int){ a = a + x; b = b + x }
   f(10)
   print a + b  # prints 23
   ```

---

## Future Work
- Break/return from lambda (`@exit_status`)
- `@the_self` for method context
- Nested lambda captures (`CaptureFwd`)
- Method type argument captures (`CaptureMethodTyArg`)
