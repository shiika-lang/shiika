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
