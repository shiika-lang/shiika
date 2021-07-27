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
    @name = name
    var @age = age
  end
end

taro = Person.new("Taro", 20)
p taro.name #=> "Taro"
p taro.age  #=> 20
taro.age += 1
```

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

