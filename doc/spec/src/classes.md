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
