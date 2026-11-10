# Modules

A module is a collection of methods which can be included into classes.
It is similar to Ruby's modules, or interfaces with default methods in
other languages.

## Module definition

Example

```sk
module Greetable
  requirement name -> String

  def greet
    puts "Hello, #{name}!"
  end
end
```

A `requirement` declares a method that the including class must define.
Methods defined in the module (like `greet` above) can call the
requirements.

## Including a module

Write module names after `:` in a class definition.

```sk
class Person : Greetable
  def initialize(@name: String); end
end

Person.new("Alice").greet  #=> Hello, Alice!
```

A class can include multiple modules. When the class also has a
superclass, the superclass must come first.

```sk
class Sub : SuperClass, Module1, Module2
```

## Modules as types

A module name can be used as a type. Any instance of a class that
includes the module conforms to it.

```sk
class A : Greetable
  ...
  def self.make -> Greetable   # Read it as "returns something Greetable"
    A.new
  end
end

A.make.greet
```

## Prerequisite modules

A module can declare that it builds on other modules by listing them
after `:`. The methods of those modules can then be used in the module.

```sk
module Named
  requirement name -> String
end

module Greetable : Named
  def greet
    puts "Hello, #{name}!"   # `name` comes from Named
  end
end
```

Note that a prerequisite module is not included automatically. A class
including `Greetable` must include `Named` too (or inherit it from its
superclass):

```sk
class Person : Greetable, Named
  def initialize(@name: String); end
end
```

Forgetting `Named` is a compile error. A value of type `Greetable` can
also be passed where `Named` is expected.

See the [language spec](../../spec/src/modules.md) for the details.

## `Self`

In a module, `Self` means "the class which includes this module". It is
useful for methods which take another value of the same class:

```sk
module Eq
  requirement eq?(other: Self) -> Bool
end

class Point : Eq
  def initialize(@x: Int, @y: Int); end
  def eq?(other: Point) -> Bool      # `Self` is `Point` here
    @x == other.x and @y == other.y
  end
end
```

To call such a method generically, use a type parameter. `a` and `b`
are guaranteed to be of the same class:

```sk
class Util
  def self.same?<T: Eq>(a: T, b: T) -> Bool
    a.eq?(b)
  end
end
```

On a value of the module type (`x: Eq`), methods which take `Self` cannot
be called because the class of the argument may differ from that of `x`.

## Equatable, Comparable and Hashable

The core library has these modules:

- `Equatable`: `==` (and `!=`)
- `Comparable : Equatable`: `<`, and `>`, `<=`, `>=` defined by `<` and `==`
- `Hashable : Equatable`: `hash`

`Int`, `Float` and `String` include all of them; `Bool` includes
`Equatable` and `Hashable`; `Class` includes `Equatable`. The keys of
`Dict` must be `Hashable`.

```sk
class Version : Comparable, Equatable
  def initialize(@n: Int); end
  def ==(other: Version) -> Bool; @n == other.n; end
  def <(other: Version) -> Bool; @n < other.n; end
end

class Util
  def self.max<T: Comparable>(a: T, b: T) -> T
    if a < b then b else a end
  end
end
Util.max(Version.new(1), Version.new(2))
```

## Example: Enumerable

The core library module `Enumerable<E>` is a good example. It requires
only `each`; a class that defines `each` and includes `Enumerable` gets
`map`, `all?`, `any?` and many other methods for free.

```sk
class Array<T> : Enumerable<T>
  def each(f: Fn1<T, Void>)
    ...
  end
end
```

See `packages/core/lib/enumerable.sk` for the full definition.
