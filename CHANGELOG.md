## [v0.9.2](https://github.com/shiika-lang/shiika/compare/v0.9.1...v0.9.2) - 2026-08-26

- Empty args compiler crash  by @EruEri in https://github.com/shiika-lang/shiika/pull/516
- Rewrite type inference by @yhara in https://github.com/shiika-lang/shiika/pull/517
- Add tests for erroneous Shiika programs by @yhara in https://github.com/shiika-lang/shiika/pull/518
- impl. Result#try! by @yhara in https://github.com/shiika-lang/shiika/pull/520
- fix command name by @yhara in https://github.com/shiika-lang/shiika/pull/525
- treat '\r' as space in lexer by @aisk in https://github.com/shiika-lang/shiika/pull/526
- Specify in install.md to use the latest version of rust by @yamakoud in https://github.com/shiika-lang/shiika/pull/528
- Fix String#chars by @yhara in https://github.com/shiika-lang/shiika/pull/522
- [NITS] remove trailing spaces by @yamakoud in https://github.com/shiika-lang/shiika/pull/529
- Issue 519 method arg check by @yamakoud in https://github.com/shiika-lang/shiika/pull/527
- define Math as a module instead of class by @aisk in https://github.com/shiika-lang/shiika/pull/524
- add basic support to run ci on mac by @aisk in https://github.com/shiika-lang/shiika/pull/533
- Update .gitignore path /tests/data/hello.txt -> /tests/tmp/hello.txt by @yamakoud in https://github.com/shiika-lang/shiika/pull/535
- Handle semicolon properly by @yamakoud in https://github.com/shiika-lang/shiika/pull/534
- Bump mio from 0.8.8 to 0.8.11 by @dependabot[bot] in https://github.com/shiika-lang/shiika/pull/536
- Bump rustix from 0.38.13 to 0.38.31 by @dependabot[bot] in https://github.com/shiika-lang/shiika/pull/539
- Remove CharType::Separator and add CharType::Newline, CharType::Semicolon by @yamakoud in https://github.com/shiika-lang/shiika/pull/537
- Develop environment in Docker container by @yamakoud in https://github.com/shiika-lang/shiika/pull/541
- Allow a line starts with dot for test case 1 by @yamakoud in https://github.com/shiika-lang/shiika/pull/542
- Update develop-in-docker-container.md by @yamakoud in https://github.com/shiika-lang/shiika/pull/543
- Ban semicolon after binary op by @yamakoud in https://github.com/shiika-lang/shiika/pull/546
- Add lib/skc_async_experiment by @yhara in https://github.com/shiika-lang/shiika/pull/544
- new_runtime: Use lib/shiika_parser by @yhara in https://github.com/shiika-lang/shiika/pull/547
- new_runtime: Rename Null to Void by @yhara in https://github.com/shiika-lang/shiika/pull/548
- Refactor: implementation for starts with dot by @yamakoud in https://github.com/shiika-lang/shiika/pull/549
- Add test case 4 by @yamakoud in https://github.com/shiika-lang/shiika/pull/550
- new_runtime: Box int and bool by @yhara in https://github.com/shiika-lang/shiika/pull/551
- new_runtime: impl. if by @yhara in https://github.com/shiika-lang/shiika/pull/552
- new_runtime: Add tests by @yhara in https://github.com/shiika-lang/shiika/pull/553
- new_runtime: Support `Int#<`, etc. by @yhara in https://github.com/shiika-lang/shiika/pull/554
- new_runtime: Fix failing tests by @yhara in https://github.com/shiika-lang/shiika/pull/555
- Move verifier.rs by @yhara in https://github.com/shiika-lang/shiika/pull/556
- Add name to Expr::ArgRef by @yhara in https://github.com/shiika-lang/shiika/pull/557
- Add pass_async_env.rs by @yhara in https://github.com/shiika-lang/shiika/pull/558
- new_runtime: impl. while by @yhara in https://github.com/shiika-lang/shiika/pull/559
- Parse unary plus by @yamakoud in https://github.com/shiika-lang/shiika/pull/560
- new_runtime: Split hir and mir by @yhara in https://github.com/shiika-lang/shiika/pull/562
- new_runtime: Remove hir::Ty by @yhara in https://github.com/shiika-lang/shiika/pull/563
- new_runtime: impl. direct method calls by @yhara in https://github.com/shiika-lang/shiika/pull/564
- new_runtime: impl. constants by @yhara in https://github.com/shiika-lang/shiika/pull/565
- new_runtime: impl. Constant name resolution by @yhara in https://github.com/shiika-lang/shiika/pull/566
- ci: update old actions by @yhara in https://github.com/shiika-lang/shiika/pull/568
- new_runtime: build vtable by @yhara in https://github.com/shiika-lang/shiika/pull/567
- new_runtime: Move skc_runtime to packages/core by @yhara in https://github.com/shiika-lang/shiika/pull/570
- build(deps): bump tokio from 1.43.0 to 1.43.1 by @dependabot[bot] in https://github.com/shiika-lang/shiika/pull/569
- new_runtime: Accessing package const by @yhara in https://github.com/shiika-lang/shiika/pull/572
- Build .sk in a package by @yhara in https://github.com/shiika-lang/shiika/pull/571
- fix type of `self` by @yhara in https://github.com/shiika-lang/shiika/pull/573
- new_runtime: Insert vtable by @yhara in https://github.com/shiika-lang/shiika/pull/575
- refactor: Convert hir_to_mir functions to HirToMir struct methods by @yhara in https://github.com/shiika-lang/shiika/pull/576
- Make classes `final` by default  by @yhara in https://github.com/shiika-lang/shiika/pull/577
- Extract sk_types.rs by @yhara in https://github.com/shiika-lang/shiika/pull/578
- new_runtime: impl. virtual functoin call by @yhara in https://github.com/shiika-lang/shiika/pull/579
- Remove atty crate from dependency (unmaintained) by @yhara in https://github.com/shiika-lang/shiika/pull/580
- new_runtime: Cleanup by @yhara in https://github.com/shiika-lang/shiika/pull/581
- Chores by @yhara in https://github.com/shiika-lang/shiika/pull/582
- new_runtime: String by @yhara in https://github.com/shiika-lang/shiika/pull/584
- new_runtime: refactor FunctionName by @yhara in https://github.com/shiika-lang/shiika/pull/585
- Convert old HIR to new MIR by @yhara in https://github.com/shiika-lang/shiika/pull/586
- new_runtime: impl. ivars by @yhara in https://github.com/shiika-lang/shiika/pull/588
- new_runtime: impl. Class object by @yhara in https://github.com/shiika-lang/shiika/pull/589
- Module calls by @yhara in https://github.com/shiika-lang/shiika/pull/590
- Llvm 18 by @yhara in https://github.com/shiika-lang/shiika/pull/591
- new_runtime: Apply TCO where possible by @yhara in https://github.com/shiika-lang/shiika/pull/593
- new_runtime: Type objects by @yhara in https://github.com/shiika-lang/shiika/pull/595
- new_runtime: Refactor wtable inserter by @yhara in https://github.com/shiika-lang/shiika/pull/596
- new_runtime: fix for nested async call by @yhara in https://github.com/shiika-lang/shiika/pull/597
- build(deps): bump bytes from 1.9.0 to 1.11.1 by @dependabot[bot] in https://github.com/shiika-lang/shiika/pull/598
- new_runtime: Dump debug MIR to separate files by @yhara in https://github.com/shiika-lang/shiika/pull/600
- new_runtime: fix redundant Alloc by @yhara in https://github.com/shiika-lang/shiika/pull/599
- new_runtime: Type related refactoring by @yhara in https://github.com/shiika-lang/shiika/pull/601
- new_runtime: codegen: Use Erasure for CreateObject by @yhara in https://github.com/shiika-lang/shiika/pull/602
- fix: Call register_current_thread by @yhara in https://github.com/shiika-lang/shiika/pull/603
- new_runtime: impl. lambda captures by @yhara in https://github.com/shiika-lang/shiika/pull/604
- new_runtime: Change test format by @yhara in https://github.com/shiika-lang/shiika/pull/605
- Update rand crate by @yhara in https://github.com/shiika-lang/shiika/pull/607
- impl. break from a block by @yhara in https://github.com/shiika-lang/shiika/pull/608
- Break in while by @yhara in https://github.com/shiika-lang/shiika/pull/609
- impl. class tvar ref by @yhara in https://github.com/shiika-lang/shiika/pull/610
- Match expr by @yhara in https://github.com/shiika-lang/shiika/pull/611
- impl. method tvars by @yhara in https://github.com/shiika-lang/shiika/pull/612
- Capture method tyarg by @yhara in https://github.com/shiika-lang/shiika/pull/613
- Builtin to core by @yhara in https://github.com/shiika-lang/shiika/pull/614
- Fix unclear compile error when clang not exist by @aisk in https://github.com/shiika-lang/shiika/pull/615
- new_runtime: Compatibility fixes by @yhara in https://github.com/shiika-lang/shiika/pull/616
- new_runtime: impl. default argument by @yhara in https://github.com/shiika-lang/shiika/pull/617
- new_runtime: File class by @yhara in https://github.com/shiika-lang/shiika/pull/619
- Cleanup old runtime by @yhara in https://github.com/shiika-lang/shiika/pull/621
- Rename skc_async_experiment to skc_main by @yhara in https://github.com/shiika-lang/shiika/pull/622
- Install tagpr by @yhara in https://github.com/shiika-lang/shiika/pull/623

## v0.9.1 (2023-09-15)

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
