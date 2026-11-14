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

## Implementing modules

A class includes a module with an `implement` block in the class body.
The block contains the methods which implement the module.

```sk
class A
  implement M
    def foo -> Int
      42
    end
  end
end
```

- A class can have multiple `implement` blocks (one for each module).
  Implementing the same module twice is a compile-time error.
- Only the superclass can be listed after `:` in the class definition
  (`class A : SuperClass`). Writing a module there is a compile-time
  error.
- Every requirement of the module must be defined in its `implement`
  block. It cannot be satisfied by a method defined outside the block,
  nor by a method inherited from the superclass. An accessor defined by
  `def initialize(@foo: ...)` does not satisfy it either; define the
  method in the block (eg. `def foo -> Int; @foo; end`).
- An `implement` block without methods can be written in one line:
  `implement M; end`.
- An `implement` block may contain only instance methods (`def` and
  `native def`) which are requirements of the module or override its
  methods. Other methods must be defined outside the block.
- A method in an `implement` block is an ordinary instance method of the
  class. The block only affects the checks above.
- A subclass inherits the modules implemented by its superclass. It
  cannot `implement` the same module again; to change the behavior,
  override the methods outside of `implement`.
- `implement` cannot be used in a module (use prerequisite modules
  instead) or in an enum (not supported yet).

The implementing class gets all the methods defined in the module as its
instance methods.

## Conditional conformance

An `implement` block may be written in a `when` block (see
[Conditional methods](classes.md#conditional-methods-when)). The class
conforms to the module only when the conditions are satisfied.

```sk
class Array<T>
  when T: Equatable
    implement Equatable
      def ==(other: Array<T>) -> Bool ...
    end
  end
end
```

- `Array<X>` conforms to `Equatable` if `X` satisfies `T: Equatable`
  (eg. `Array<Int>`, `Array<Array<Int>>`, `Array<U>` for a type parameter
  `U: Comparable`), but not otherwise (eg. `Array<Object>`). The rules of
  the conditions are the same as those of the methods in `when` blocks
  (eg. `Array<Equatable>` does not conform to `Equatable` because
  `Equatable` uses `Self`).
- The methods in the `implement` block are methods of the `when` block.
  The other methods of the module (eg. `<=` of `Comparable`) can be
  called only when the conditions are satisfied, too.
- For `T == X`, `T` in the module type and `Self` are replaced with `X`
  (eg. `implement Named` in `when T == String` of `class Tagged<T>` makes
  `Tagged<String>` conform to `Named`).
- The prerequisite modules must be implemented under the same (or weaker)
  conditions. eg. `implement Comparable` in `when T: Comparable` requires
  `Box<T>` to conform to `Equatable` when `T: Comparable`.
- A module cannot be implemented twice, even with different conditions.
- A subclass inherits the conditional conformance (eg. `class Sub : Box<String>`
  conforms to `Equatable`).

## Modules as types

A module name denotes a type. An instance of a class that implements
module `M` conforms to `M`.

```sk
class A
  implement M
    ...
  end
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

class Array<T>
  implement Enumerable<T>
    def each(f: Fn1<T, Void>) ...
  end
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

`module M1 : M2` means "a class which implements `M1` must also conform to
`M2`".

- Implementing `M1` does **not** implement `M2` automatically. The
  class (or one of its ancestors) must have `implement M2` by itself;
  otherwise it is a compile-time error.

  ```sk
  class Int                 # ok
    implement Comparable<Int> ... end
    implement Equatable<Int> ... end
  end
  class Foo                 # error: Comparable<Foo> requires Equatable<Foo>
    implement Comparable<Foo> ... end
  end
  ```

  This is checked with conformance, so the prerequisite may be satisfied
  by a superclass or via variance (eg. `Dog` conforms to
  `Equatable<Dog>` if its superclass `Animal` implements
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
refers to the type which implements the module.

```sk
module Eq
  requirement eq?(other: Self) -> Bool

  def neq?(other: Self) -> Bool
    not eq?(other)
  end
end

class Point
  implement Eq
    def eq?(other: Point) -> Bool   # `Self` is `Point` here
      ...
    end
  end
end
```

- `Self` cannot be used outside the instance methods and the method
  requirements of a module (eg. in a class, or in a class method of a
  module). It does not take type arguments.
- In a method requirement, `Self` may only appear in contravariant
  positions (eg. parameter types, but not return types nor `Array<Self>`).
- When checking that a class satisfies the requirements of a module,
  `Self` is the type of the class which has the `implement` block.
  A subclass of it which overrides the method must accept that class,
  because the method may be called via that class.

  ```sk
  base class Shape
    implement Eq
      def eq?(other: Shape) -> Bool; ...; end
    end
  end
  class Square : Shape
    def eq?(other: Square) -> Bool; ...; end   # error; must accept `Shape`
  end
  ```
- When calling a method of a module, `Self` in its signature is replaced
  as follows.
  - On a value of a class: `Self` is the class which implements the module
    (eg. for `x: Dog` where `Animal` implements `Eq`, `x.neq?(y)` requires `y: Animal`).
  - On a value of a type parameter whose upper bound is the module
    (`x: T` where `T: Eq`): `Self` is the type parameter, so `x.neq?(y)`
    requires `y: T`.
  - In the instance methods of a module, `self` is of type `Self`.
    Therefore `self` can be passed where `Self` is expected, but a value of
    the module type cannot (it may be of a different class than `self`).
  - On a value of the module type (`x: Eq`): the actual class of `x` is
    unknown. `Self` in the return type is the module type. A method which
    takes `Self` as a parameter type cannot be called (only `Never` could
    be passed safely); use a type parameter (`T: Eq`) instead. A method
    which has `Self` in other positions (eg. returns `Array<Self>`) cannot
    be called either.

    ```sk
    def f(x: Named) -> String
      x.greeting      # ok; no `Self` in the signature
      x.me.name       # ok; `me -> Self` returns `Named`
    end
    def g(x: Eq, y: Eq) -> Bool
      x.eq?(y)        # error; `x` and `y` may be of different classes
    end
    ```
  - The same applies to the methods of the prerequisite modules.
- A module type cannot be a type argument of a type parameter whose upper
  bound is a module which uses `Self` (in its own methods or its
  prerequisite modules), even though the module type conforms to the
  bound. For example, given `def max<T: Comparable>(a: T, b: T) -> T`,
  `max<Comparable>(x, y)` is an error because `a < b` in `max` assumes
  `a` and `b` are of the same class. This applies to explicit, inferred
  and class type arguments (eg. `Box<Comparable>` for `class Box<T: Comparable>`).
  A type parameter (`U: Comparable`) or `Never` can be passed.
- `Self` is reserved; it cannot be used as the name of a class, a module
  or an enum.
