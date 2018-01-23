# Shiika language spec

(Everything here is draft)

## Lexical structure

TBA

## Types

- Every value in Shiika is an object and belongs to a class (like Int, String, etc.)

Classes

- A class has:
  - 0 or more type parameters
    - eg. `class Stack[T]` has one type parameter
  - 0 or more instance variables
  - 0 or more instance methods
    - Note: all methods are `public`. It is encouraged to prefix `_` for "private" ones.
  - 0 or more class methods
  - 0 or more constants
  - A superclass template (described below) and 0 or 1 superclass
- A class can `include` 0 or more modules
  By including a module M, the class will
  - have M's instance variables
  - have M's instance methods
  - have M's constants
- A class can `extend` 0 or more modules
  By extending a module M, the class will
  - have M's instance variables
  - have M's instance methods as its class methods

### Generics

Generic classes

- If a class has 1 or more type parameters, that class is called a *generic class*.
- If a class has no type parameters, that class is called a *non-generic class*.

Specialized classes

- Conceptually, defining a generic class `Stack[T]` is defining infinite set of classes like
  `Stack[Int]`, `Stack[Array[Int]]`, `Stack[Stack[Int]]`.
  - These classes are called *specialized class* of `Stack[T]`.
  - `Int`, `Array[Int]`, `Stack[Int]` here are called a *type argument*.
  - Type arguments must not be a generic class.
  - Specialized class cannot have a type parameter. In this sense, specialized class is non-generic.

Superclass and superclass template

- A class has a superclass template.
  - For `class A < B`, `B` is the superclass template.
  - For `class A[T] < B`, `B` is the superclass template with no type patemeters.
  - For `class A[T] < B[Int]`, `B[Int]` is the superclass template with no type patemeters.
  - For `class A[T] < B[T]`, `B[T]` is the superclass template, with a type parameter `T`.
    - A superclass template may refer type parameters of the base class.
- Given a class C:
  - If C is non-generic, the superclass of C is the class described by the superclass template.
  - If C is generic but the superclass template does not have type parameters,
    that class is the superclass of the generic class and any specialized version of it.
  - When the superclass template of a generic class has type parameters:
    - Superclasses of specialized class of C is determinted by the superclass template.
    - In this case, C itself does not have a superclass.

### Metaclass

- When class `A` is defined, a class `Meta:A` is automatically defined.
  `Meta:A` is called the *metaclass* of `A`.
- Metaclass of `Stack[T]` is `Meta:Stack[T]`.
  Metaclass of a generic class is also generic.
- Metaclass of `Stack[Int]` is `Meta:Stack[Int]`.
  Metaclass of a specialized class is also specialized.

## Program structure

- Definitions
  - Class definition
    - Method definition
    - Initializer definition (can specify `@foo` in the paremeter list)
    - Constant declaration
- Expressions
  - Conditional
    - `if` expression
  - Invocation
    - Method call
    - Function call
  - Assignment
    - Local variable assignment
    - Instance variable assignment
  - Values
    - Local variable reference
    - Instance variable reference
    - Constant reference
    - Literals
      - Integer
      - Float 
      - Bool
      - nil
- Statements
  - `return` statement

## Typing rule

- `if <cond-expr> then <then-expr> else <else-expr> end`
  - The type of `cond-expr` must be Bool
  - For `then-expr` and `else-expr`, type of 
