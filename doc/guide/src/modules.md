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

A `requirement` declares a method that the class must define.
Methods defined in the module (like `greet` above) can call the
requirements.

## Implementing a module

Write an `implement` block in a class definition, and define the
requirements of the module in it.

```sk
class Person
  def initialize(@first: String, @last: String); end

  implement Greetable
    def name -> String
      "#{@first} #{@last}"
    end
  end
end

Person.new("Alice", "Liddell").greet  #=> Hello, Alice Liddell!
```

This makes clear which methods are there for the module. A class can
implement multiple modules with multiple `implement` blocks.

```sk
class Sub : SuperClass
  implement Module1
    ...
  end
  implement Module2
    ...
  end
end
```

A subclass inherits the modules implemented by its superclass.

## Modules as types

A module name can be used as a type. Any instance of a class that
implements the module conforms to it.

```sk
class A
  implement Greetable
    ...
  end
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

Note that a prerequisite module is not implemented automatically. A class
implementing `Greetable` must implement `Named` too (or inherit it from its
superclass):

```sk
class Person
  def initialize(@name: String); end

  implement Greetable; end

  implement Named
    def name -> String
      @name
    end
  end
end
```

Forgetting `Named` is a compile error. A value of type `Greetable` can
also be passed where `Named` is expected.

See the [language spec](../../spec/src/modules.md) for the details.

## `Self`

In a module, `Self` means "the class which implements this module". It is
useful for methods which take another value of the same class:

```sk
module Eq
  requirement eq?(other: Self) -> Bool
end

class Point
  def initialize(@x: Int, @y: Int); end

  implement Eq
    def eq?(other: Point) -> Bool    # `Self` is `Point` here
      @x == other.x and @y == other.y
    end
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

`Int`, `Float` and `String` implement all of them; `Bool` implements
`Equatable` and `Hashable`; `Class` implements `Equatable`. The keys of
`Dict` must be `Hashable`. `Array<T>` is `Equatable` when `T` is
`Equatable` (see below), and has `sort` when `T` is `Comparable` (see
`when` in [Classes](classes.md)).

Only the types which implement `Equatable` have `==`. To check if two
values are the same object, use `identical?` (defined on all objects).

```sk
class Foo; end
let a = Foo.new
a == a            # error; Foo does not implement Equatable
a.identical?(a)   #=> true
```

```sk
class Version
  def initialize(@n: Int); end

  implement Comparable
    def <(other: Version) -> Bool; @n < other.n; end
  end

  implement Equatable
    def ==(other: Version) -> Bool; @n == other.n; end
  end
end

class Util
  def self.max<T: Comparable>(a: T, b: T) -> T
    if a < b then b else a end
  end
end
Util.max(Version.new(1), Version.new(2))
```

## Conditional conformance

A class can implement a module only when its type arguments satisfy some
conditions, by writing `implement` in a `when` block:

```sk
class Box<T>
  def initialize(@v: T); end

  when T: Equatable
    implement Equatable
      def ==(other: Box<T>) -> Bool
        @v == other.v
      end
    end
  end
end

p Box.new(1) == Box.new(1)        #=> true
p Box.new([1]) == Box.new([1])    #=> true
Box.new(Object.new) == Box.new(Object.new)   # error; Object is not Equatable
```

`Array` is `Equatable` in this way, so `[[1, 2], [3]] == [[1, 2], [3]]`
works and `Array<Int>` can be passed where `T: Equatable` is expected.

## Example: Enumerable

The core library module `Enumerable<E>` is a good example. It requires
only `each`; a class that implements `Enumerable` with `each` gets
`map`, `all?`, `any?` and many other methods for free.

```sk
class Array<T>
  implement Enumerable<T>
    def each(f: Fn1<T, Void>)
      ...
    end
  end
end
```

See `packages/core/lib/enumerable.sk` for the full definition.
