# Shiika language spec

(Everything here is draft)

## Lexical structure

TBA

## Types

- Every value in Shiika is an object and belongs to a class (like Int, String, etc.)

Classes

- A class has:
  - 0 or more type parameters
    - `class Stack<T>
  - 0 or more instance variables
  - 0 or more instance methods
    - Note: all methods are `public`. It is encouraged to prefix `_` for "private" ones.
  - 0 or more class methods
  - 0 or more constants
- A class can `include` 0 or more modules
  By including a module M, the class will
  - have M's instance variables
  - have M's instance methods
  - have M's constants
- A class can `extend` 0 or more modules
  By extending a module M, the class will
  - have M's instance variables
  - have M's instance methods as its class methods

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
