# CodeGen

Directory: `lib/skc_codegen`

CodeGen generates LLVM IR from Shiika HIR.

## Files

`mod.rs` is the entry point of CodeGen. `gen_exprs.rs` contians the functions of CodeGen which handles Shiika expressions.

## Dependency

Shiika uses [inkwell](https://github.com/TheDan64/inkwell) to generate LLVM IR.

## Classes

For each class, a LLVM type is defined.

```llvm
%Array = type { i8*, %Int*, %Int*, %"Shiika::Internal::Ptr"* }
```

Two `%Int*` corresponds to `@capa` and `@n_items`. The last `%"Shiika::Internal::Ptr"*` corresponds to `@items`.

The first `i8*` points to the vtable of the object. In the case of `Array`, it points to the LLVM constant `@vtable_Array`.

### Primitives

In the case of `Array`, the three instance variables are all Shiika object. However some of the core classes contains LLVM value instead of a Shiika value. Here is the list:

```llvm
%Int = type { i8*, i64 }
%Float = type { i8*, double }
%Bool = type { i8*, i1 }
%"Shiika::Internal::Ptr" = type { i8*, i8* }
```

### Metaclasses

TBA

```llvm
%"Meta:Float" = type { i8*, %String* }
%"Meta:Int" = type { i8*, %String* }
```

### Constants

For each constant, a LLVM constant is defined. Constants are initialized with `null` at first and initialized by `@init_constants()`.

```llvm
@"::Array::INITIAL_CAPA" = internal global %Int* null
```

Note that there are constants that holds a class object. For example the constant `::String` is defined as below.

```llvm
@"::String" = internal global %"Meta:String"* null
```

## Methods

For each method, a LLVM function is defined. For example:

```llvm
define %Int* @"String#bytesize"(%String* %self) {
  %addr_bytesize = getelementptr inbounds %String, %String* %self, i32 0, i32 2
  %bytesize = load %Int*, %Int** %addr_bytesize
  ret %Int* %bytesize
}
```

The first argument is the receiver object (in the above case, a String.) Method arguments follow it if any.

## Lambdas

Instances of `Fn1`, `Fn2`, ... are called "lambda" in Shiika. There are two ways to create lambda.

1. Lambda expression `fn(x){ ... }`
2. Passing block to a method `ary.each{ ... }`

In both cases, a llvm function is defined for it. For example:

```llvm
define %Object @lambda_1(%Fn1* %fn_x, %Object* %arg1, %Object* %arg2) {
  ...
}
```

Note that all arguments and return value are handled as `%Object*`, regardless of their original type. This is because all lambdas that accepts one argument are invoked by `Fn1#call`.

### FnX

`Fn1`, `Fn2`, ... are the classes for lambdas. It has three instance variables:

- `@func` is a pointer to `@lambda_xx`.
- `@the_self` is the object pointed by `self` in the lambda.
- `@captures` is an array of outer variables the lambda captures.
