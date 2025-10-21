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
    let @title = title
    let @price = price
  end
end
```

For convenience, this can be written as:

```sk
class Book
  def initialize(@title: String, @price: Int); end
end
```

Instance variables are read-only by default. To make it reassignable, declare it with `var`.

## Accessors

For each instance variable, accessor methods are automatically defined. A reader method for an read-only one, reader and setter method for an writable one.

Example

```sk
class Person
  def initialize(name: String, age: Int)
    let @name = name
    var @age = age
  end
end

taro = Person.new("Taro", 20)
p taro.name #=> "Taro"
p taro.age  #=> 20
taro.age += 1

taro.name = "Jiro" # This is error because @name is not declared with `var`.
```

## Visibility

Shiika does not have visibility specifier like `private` or `protected`. Conventionally, it is preferred to prefix `_` for instance variables which are intended "internal".

```sk
class Person
  def initialize(name: String, age: Int)
    let @name = name
    var @age = age
    let @_secret_count = 0
  end
end
```

In this case `Person.new._secret_count` is valid but normally you should avoid this because it is considered "private". Private method of a library is rather an implementation detail than public API. It may be silently changed in the future version of the library.

Shiika allows this for in case you _really_ need it.

## Classes and metaclasses

(Usually you don't need to care about this topic. This section is written in case you are curious)

In Shiika, classes are objects too. For example, constant `::Int` holds the _class object_ of the class `Int`.

```sk
p Int #=> #<class Int>
```

Every object has `.class` method which returns the class object of its class.

```sk
p 123.class        #=> #<class Int>
p 123.class == Int #=> #<class Int>
```

So what happens if you call `.class` on `Int`? Let's see.

```sk
p Int.class        #=> #<class Meta:Int>
```

`Int` belongs to a secret class named `Meta:Int`. This class is called _metaclass_ of `Int` and has the class methods of `Int`.

This _metaclass object_ also belongs to a class `Metaclass`. But this relationship is not infinite because the class of `Metaclass` is defined as itself.

```sk
p Int.class.class        #=> #<class Metaclass>
p Int.class.class.class  #=> #<class Metaclass>
```

Last but not least, don't confuse this class-instance relationship with class inheritance (in other word, subclass-superclass relationship.) In the figure below, class-instance relationship is shown horizontally. Inheritance is shown vertically with `^`.

```
~ ... class-instance relationship
^ ... superclass-subclass relationship

                   Object
                      ^
                    Type              Class        Class
                      ^                 ^            ^                            
                    Class          ~ Meta:Class  ~ Metaclass       Type     ~ Meta:Type       ~ Metaclass
                      ^                                             ^
     Object     ~ Meta:Object      ~ Metaclass                    Module    ~ Meta:Module
        ^                                                           ^              
123 ~  Int      ~   Meta:Int       ~ Metaclass                     Math     ~ Meta:Math
```

For each class, there is a class object

```
                      Object               Class                      Class
                        ^                    ^                          ^
                      Class             ~ Meta:Class              ~ Metaclass
                   #<class Class>      #<metaclass Meta:Class>
                        ^
                        |
     Object     ~   Meta:Object          ~ Metaclass
  #<class Object> #<metaclass Meta:Object>
        ^               ^ 
        |               |             
       Int      ~     Meta:Int           ~  Metaclass
    #<class Int>  #<metaclass Meta:Int>    #<metaclass Metaclass>

```
