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
  def self.make -> Greetable
    A.new
  end
end

A.make.greet  # Calls `A#greet` (inherited from Greetable)
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
