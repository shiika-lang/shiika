## v0.5.3 (2021-01-11)

- feat: Added many methods
- feat: Impl. `Array<T>.new` (#222)
- feat: if/unless modifier (#61)
- feat: String interpolation (#218)
- feat: Support `initialize(@a: Int)` like Crystal (#211)
- feat: class Hash (#232)
- feat: Refine backtrace (f56eabe)
- misc: Upgrade to LLVM 9 (#219)
- fixes: #179, b00ce1c, #226, #230, #234

## v0.5.2 (2020-12-27)

- Breaking change: Rename `Array#nth` to `Array#[]` (#155)
- feat: Added many methods
- feat: `\n`, etc. (#190)
- feat: Int is now 64bit (#198)
- feat: Better parse error (1262091)
- feat: elsif (#201)
- fixes: #183, #184, #194, #196, #197, #199, #200, #214

## v0.5.1 (2020-12-04)

- feat: Block syntax (#173)
- feat: `-=`, etc. (#165)
- fixes: #176 #175 

## v0.5.0 (2020-11-06)

- feat: Virtual methods (#166)
- feat: `p` and `inspect` (#168)
- feat: `+=` (#163)
- feat: `panic`, `exit` (#162)

## v0.4.0 (2020-09-06)

- feat: [Anonymous function](https://github.com/yhara/shiika/projects/2)
- feat: Now you don't need Ruby to build shiika (#148)
- fixes: #130 #138

## v0.3.0 (2020-07-28)

- feat: Array literal (#84)
- feat: Basic generics like Array#first (#101)
- fixes: #113 #114 #118

## v0.2.5 (2020-05-29)

- feat: Automatically define getters/setters for instance variables (#44)
- feat: Specify superclass (#70)
  - Inherit superclass ivars (#73)
- feat: Inner class definition (#69)
- feat: `unless` (#66)
- fixes: #68 #62 #55
- chore: Update inkwell (#65)

## v0.2.4 (2020-05-06)

- New example: ray
- feat: Mutable ivar (#45)
- feat: Class#name (#33)
- feat: Logical operators (#16)
- Bug fixes, add some methods

## v0.2.3 (2020-03-19)

- New examples: mandel, hello
- feat: String literal and `puts` (#9)
- feat: Support `if` with multiple stmts (#4)
- fix: Parse a*b*c (#5)

## v0.2.2 (2019/12/17)

- feat: shiika compile, shiika run
- feat: while expression

## v0.2.1 (2019/11/06)

- New example: fib
- feat: One-line comment
- feat: Add some operators
- feat: Constant

## v0.2.0 (2019/07/17)

- Started reimplementation with Rust

## v0.1.3 (2019-05-20)

- `-> Void` is now optional
- Type checker: supports inheritance

## v0.1.2 (2018-06-16)

- Array literal

## v0.1.1 (2018-06-13)

- varargs
- Array class

## v0.1.0 (2018-06-07)

- Basic generics

## v0.0.2 (2017-12-19)

- ivar reference

## v0.0.1 (2017-12-17)

- instance creation

## v0.0.0 (2017-11-09)

- initial commit
