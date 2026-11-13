# Classes

## Class definition

Example

```sk
class A
  # A class method
  def self.foo -> Int
    1
  end

  # An instance method
  def bar -> Int
    2
  end
end

p A.foo #=> 1
p A.new.bar #=> 2
```

### Builtin comparison methods

`Int#<`, `Int#<=`, `Int#>`, `Int#>=`, `Int#==` and `Bool#==` are compiler
intrinsics. A package importing their core definitions cannot redefine these
methods. Calls with concrete Int/Bool operands may compile directly to primitive
comparisons; calls through modules or type parameters retain method
dispatch. In either case operands are evaluated once, from left to right.

## Instance variables

Name of an instance variable starts with `@`. All instance variables of a class must be initialized in the method `initialize`.

Example

```sk
class Book
  def initialize(title: String, price: Int)
    @title = title
    @price = price
  end
end
```

Syntax sugar:

```sk
class Book
  def initialize(@title: String, @price: Int); end
end
```

Instance variables are readonly by default. To make it reassignable, declare it with `var`.

## Accessors

For each instance variable, accessor methods are automatically defined unless they are defined explicitly.

Example

```sk
class Person
  def initialize(name: String, age: Int)
    let @name = name
    var @age = age
  end
end

let taro = Person.new("Taro", 20)
p taro.name #=> "Taro"
p taro.age  #=> 20
taro.age += 1
```

## Inheritance

```sk
base class Base1
  def foo -> Int
    1
  end
end

class Sub1 : Base1
end

p Sub1.new.foo  #=> 1
```

The supertypes of a class are listed after `:` in the class
definition.

```sk
class Sub : SuperClass, Module1, Module2
```

- At most one superclass is allowed, and it must be the first item of
  the list. The other items must be [modules](./modules.md).
- The superclass must be declared with the `base` keyword
  (`base class`). Inheriting a class not declared as `base` is a
  compile-time error.
- A type parameter cannot be a supertype.
- The subclass inherits the methods, `initialize` and instance
  variables of the superclass. Defining a method of the same name
  overrides it.
- When no superclass is given, the superclass is `Object`.

## Constants

An assignment to a name starting with an uppercase letter defines a
constant. Reassigning to a constant is a compile-time error.

```sk
FOO = 1
```

A constant defined in a class body belongs to the class. Constants are
resolved from the innermost namespace outward, and can be referred to
with a qualified name using `::`.

```sk
class A
  BAR = 2

  class B
    def self.baz -> Int
      BAR  # resolved to ::A::BAR
    end
  end
end

p A::BAR  #=> 2
```

(In this document, the notation `::A::BAR` is used to denote the full
name of a constant, though the leading `::` cannot be written in
programs currently.)

Note that a class definition `class A ... end` also defines the
constant `::A`, which holds the class object of `A`.

## Class hierarchy

```
^ ... superclass-subclass relationship
~ ... class-instance relationship

               Object       Object       Object
                  ^            ^            ^
                Class     ~ MetaClass  ~ MetaClass
                  ^
     Object ~ Meta:Object ~ MetaClass
        ^         ^ 
        |         |       
        |         |        
123 ~  Int ~   Meta:Int   ~ MetaClass
```

Example:

```sk
p 123                   #=> 123
p 123.class             #=> #<class Int>
p Int                   #=> #<class Int>
p 123.class == Int      #=> true

p Int.class             #=> #<class Meta:Int>
p Int.class.class       #=> #<class Metaclass>
p Int.class.class.class #=> #<class Metaclass>
```

## Conditional methods (`when`)

A `when` block in a class or module body defines methods which are
available only when the type arguments satisfy the conditions.

```sk
class Box<T>
  when T: Comparable
    def max_item -> T ...      # `T` is bounded by `Comparable` here
  end
  when T == String
    def join_all -> String ...  # `T` is `String` here
  end
end
```

- A condition is either `T: X` (the type argument of `T` conforms to `X`)
  or `T == X` (the type argument of `T` is exactly `X`). Multiple
  conditions are separated by `,` (all of them must be satisfied).
  `T` must be a type parameter of the class or module. `X` cannot contain
  type parameters (not supported yet).
- In the block, `T: X` makes `X` the upper bound of `T`, and `T == X`
  makes `T` an alias of `X` (eg. `self` is `Box<String>`).
- Calling a method of a `when` block is an error unless the conditions are
  satisfied by the type arguments of the receiver.
  - For `T: X`, `Never` satisfies it. A module type cannot be used for a
    condition whose `X` uses `Self` (same as the bound of a type parameter).
  - For `T == X`, the type argument must be `X` itself (`Never` does not
    satisfy it). A type parameter `U` does not satisfy it either, unless
    the caller is also in a `when U == X` block.
- Only instance methods (`def`, `native def`) can be defined in a `when`
  block (not `initialize`, class methods, `implement`, etc.).
- A method in a `when` block cannot have the same name as another method
  of the class or module.
- A method which overrides a method of a `when` block must be in a `when`
  block of the same conditions.

## Native methods

A package which has `rust_libs` in its `package.json5` can declare methods
implemented in Rust with `native def`. A native method has only a signature
(no body and no `end`).

```sk
class Int
  # Returns `self` + `other`
  native def +(other: Int) -> Int
end
```

- Use `native_async def` when the Rust function is async.
- A native method can be an instance method or a class method (`native def self.foo`).
- Parameters of a native method cannot be instance variables (eg. `native def initialize(@a: Int)` is an error).
- A class cannot have a `def` and a `native def` with the same name.
- `native def` in a package without `rust_libs` (including a program compiled
  by `shiika compile`) is an error.
