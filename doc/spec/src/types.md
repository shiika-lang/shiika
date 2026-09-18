# Types

Shiika is a statically-typed language. Every expression has a type
which is determined at compile time.

## Term types

These forms of types exist:

- Class types (`Int`, `String`, ...)
  - Instantiated generic class types (`Array<Int>`, `Fn1<Int, Bool>`, ...)
- Module types (`Enumerable<Int>`, ...)
- Metaclass types (`Meta:Int`, `Metaclass`, ...); the type of class objects
- Type parameter references (the type of `x` in
  `def foo<T>(x: T)` is a reference to `T`)

## Special types

### Object

`Object` is the top type; every class is a descendant of `Object`.

### Void

`Void` is the type of expressions which have no meaningful value, such
as `while` or a call of a method without return type. A value of type
`Void` cannot be used as an argument or a receiver.

### Never

`Never` is the bottom type. It is the type of expressions that never
return a value, such as `return`, `break` or a call of
`Object#panic`. `Never`
conforms to every type; an expression of type `Never` can be placed
where any type is expected. No value of `Never` exists at runtime.

## Conformance

A value of type `S` can be used where type `T` is expected when `S`
_conforms to_ `T`. `S` conforms to `T` if:

- `S` equals `T`, or
- `S` is `Never`, or
- `T` is an ancestor of `S`, i.e. one of:
  - the superclass of `S` (transitively),
  - a module included by `S` or its ancestors.

Note that generic types are invariant by default: `Array<Int>` does
not conform to `Array<Object>`.

### Variance annotations

A type parameter may be declared covariant with `out` or contravariant
with `in`.

```sk
class Foo<out T>
end
```

## Type parameters

Classes, modules and methods can take type parameters (see
[Generics in the guide](../../guide/src/generics.md)). Within their
body, a type parameter can be used like a type. The upper bound of a
type parameter is `Object`; only the methods of `Object` can be called
on a value of a type parameter type.
