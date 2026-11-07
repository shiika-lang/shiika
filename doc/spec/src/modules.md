# Modules

## Module definition

```sk
module M
  requirement foo -> Int

  def bar -> Int
    foo + 1
  end
end
```

A module definition consists of _requirements_ and method definitions.

- `requirement` declares a method signature. It may only appear in
  module definitions.
- Methods defined in the module may call the requirements (and the
  other methods of the module).

A module itself cannot be instantiated.

## Including modules

A class includes modules by listing them after `:` in the class
definition.

```sk
class A : M
  def foo -> Int
    42
  end
end
```

- Multiple modules can be included:
  `class A : M1, M2`.
- When the class has a superclass, it must come before the modules:
  `class A : SuperClass, M1, M2`. Listing a class anywhere else is a
  compile-time error.
- The including class must define all the requirements of the modules
  (either directly or via its superclass).

The including class gets all the methods defined in the module as its
instance methods.

## Modules as types

A module name denotes a type. An instance of a class that includes
module `M` conforms to `M`.

```sk
class A : M
  ...
  def self.bar -> M
    A.new  # ok; A conforms to M
  end
end
```

When a method is called on a value whose static type is a module type,
the implementation of the actual class is invoked (dynamic dispatch;
implemented with witness tables. See the
[Shiika Hacking Guide](../../shg/src/architecture.md) for internals).

## Generic modules

Modules can take type parameters.

```sk
module Enumerable<E>
  requirement each(f: Fn1<E, Void>) -> Void
  ...
end

class Array<T> : Enumerable<T>
  ...
end
```

## Prerequisite modules

A module may list other modules after `:` in its definition. They are
_prerequisite modules_ of the module.

```sk
module Equatable<in X>
  requirement ==(other: X) -> Bool
end

module Comparable<in X> : Equatable<X>
  requirement <(other: X) -> Bool

  def <=(other: X) -> Bool
    self == other or self < other   # ok; `==` of Equatable
  end
end
```

`module M1 : M2` means "a class which includes `M1` must also conform to
`M2`".

- Including `M1` does **not** include `M2` automatically. The including
  class (or one of its ancestors) must include `M2` by itself; otherwise
  it is a compile-time error.

  ```sk
  class Int : Comparable<Int>, Equatable<Int>  # ok
  class Foo : Comparable<Foo>                  # error: Comparable<Foo> requires Equatable<Foo>
  ```

  This is checked with conformance, so the prerequisite may be satisfied
  by a superclass or via variance (eg. `Dog` conforms to
  `Equatable<Dog>` if its superclass `Animal` includes
  `Equatable<Animal>` and the type parameter of `Equatable` is `in`).
- `M1<A>` conforms to the prerequisite modules of `M1` (with the type
  arguments substituted), transitively. Therefore the methods of `M2`
  can be called on a value of type `M1` (including `self` in the method
  definitions of `M1`, and a value of a type parameter whose upper bound
  is `M1`).
- Only modules can be listed. A class or a type parameter is not
  allowed, nor is the module itself.
- A prerequisite module must be defined before the module.
- A module must not define a method whose name is also defined in its
  prerequisite modules (transitively). Prerequisite modules must not
  have methods of the same name with each other, unless they come from
  the same module (eg. `M1 : M2, M3` where both `M2` and `M3` require
  `M0`).
- The same module must not be required with different type arguments
  (eg. both `M0<Int>` and `M0<String>`).

## `Self`

In the instance methods and the method requirements of a module, `Self`
refers to the type which includes the module.

```sk
module Eq
  requirement eq?(other: Self) -> Bool

  def neq?(other: Self) -> Bool
    not eq?(other)
  end
end

class Point : Eq
  def eq?(other: Point) -> Bool   # `Self` is `Point` here
    ...
  end
end
```

- `Self` cannot be used outside the instance methods and the method
  requirements of a module (eg. in a class, or in a class method of a
  module). It does not take type arguments.
- In a method requirement, `Self` may only appear in contravariant
  positions (eg. parameter types, but not return types nor `Array<Self>`).
- When checking that a class satisfies the requirements of a module,
  `Self` is the type of the class which includes the module. The
  requirement may also be satisfied by a method of a superclass; in this
  case `Self` is that superclass.

  ```sk
  base class Animal
    def eq?(other: Animal) -> Bool; ...; end
  end
  class Dog : Animal, Eq   # ok; satisfied by `Animal#eq?`
  end
  ```

  On the other hand, a subclass of the includer which overrides the method
  must accept the includer, because the method may be called via the
  includer.

  ```sk
  base class Shape : Eq
    def eq?(other: Shape) -> Bool; ...; end
  end
  class Square : Shape
    def eq?(other: Square) -> Bool; ...; end   # error; must accept `Shape`
  end
  ```
- `Self` is reserved; it cannot be used as the name of a class, a module
  or an enum.
