## v0.9.0 (2023-09-15)

- Setup
  - Upgrade to llvm-16 (#507)
- Syntax
  - feat: Allow omitting type arguments on .new and method-wise type arguments (#491, #492, #493)
  - fix: `if` and `while` now makes a local variable scope (#388 - Thank you @EruEri!)
- Library
  - new: `Object#sleep`, `Shiika::Internal::Memory.force_gc` (#498)
  - new: `File.open`, `Readable` (#496)

## v0.8.0 (2023-04-25)

- Breaking changes
  - Require `let` for variable declaration (#418)
- Syntax
  - feat: Method call with keyword arguments (#464)
  - feat: Default value for method arguments (#469)
  - feat: Add syntax to call a function stored in ivar (#420)
- Library
  - new: `Object#gets` (#457), `Random` (#456), `File.read`, `File.write` (#450)
- Other
  - feat: Windows support (#449)
  - feat: `SHIIKA_ROOT` to specify builtin path (#461)
  - bug fixes

## v0.7.1 (2022-11-04)

- Syntax
  - feat: Method type parameter inference (#415)
  - feat: Block parameter type inference (#389)
  - feat: Class-level initialize (#392)
  - feat: Allow bool/float/string literal in pattern match (#383)
- Library
  - feat: `Time` class (#413)
  - feat: String#chars supports emojis (#375, #377)
- Other
  - feat: Show backtrace on panic if `RUST_BACKTRACE=1` is set (#386)
  - feat: Support stable Rust (#370)
  - feat: Show source location on some errors (#367, etc.)
  - bug fixes

## v0.7.0 (2022-07-12)

- feat: Modules (#346)
- feat: require (#366)
- Breaking change: Renamed `build_corelib` subcommand to `build-corelib`
- Breaking change: Replace `!` with `not` (#351)
- Breaking change: Remove conditional operator (`? :`) (#357)
- feat: Object#unsafe_cast (#363)
- feat: Added many core methods

## v0.6.1 (2022-03-02)

- Breaking change: `Int#/` now returns Float (#331)

## v0.6.0 (2022-02-27)

- feat: Basic pattern matching (#306)
- feat: Class alias (#296)
- misc: Added class Metaclass (#301)
- internal: Split the compiler into several crates under lib/* (#316)
- feat: `Object#class` (#299), `Float#to_s` (#320), etc.
- Breaking change: Renamed `is_empty` to `empty?`, etc. (#315)
- Breaking change: Renamed `Hash` to `Dict` (#321)
- Breaking change: Renamed `Int#&` to `Int#and`, etc. (#329)

## v0.5.5 (2021-07-12)

- Breaking change: Changed build process (see README) (#280, #284)
- feat: Enums (#142)
- feat: Inheritance from generic class (#287)
- feat: Relative reference of constants (#285) and type names (#291)

## v0.5.4 (2021-04-04)

- Breaking change: Moved repository to shiika-lang/shiika. Also renamed
  branch `master` to `main`
- Breaking change: Removed `Fn#call`. Use `f()` instead (#269)
- Breaking change: Removed `&&/||`. Use `and/or` instead (#278)
- feat: Generic methods like `Array<T>#map<U>` (#237)
- feat: `return` (#263)
- feat: `Array#[]`, `[]=` now accepts negative index (f3898fa)
- feat: `Class#inspect` now works (#247)
- feat: When `if` branches has different type, the type of `if` is the
  nearest common ancestor of them (#274)
- feat: Added more methods

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

- feat: [Anonymous function](https://github.com/shiika-lang/shiika/projects/2)
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
