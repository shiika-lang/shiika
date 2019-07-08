# Shiika

A statically-typed programming language.

## Example

        class Pair<S, T>
          def initialize(@a: S, @b: T)
          end

          def fst -> S; @a; end
          def snd -> T; @b; end
        end
        Pair<Int, Bool>.new(1, true).fst

## Status

Early-alpha

### Implementation (Rust, src/*)

- [x] Parse and emit LLVM IR for minimal case 
- [ ] Check type of return value
- [ ] Implement metaclass (eg. `A.new`)
- [ ] Nested class
- [ ] Constant

### Type system (Prototype in Ruby, lib/*)

#### Done

- [x] Class method, instance method
- [x] Basic generics
- [x] Variable-length arguments
- [x] Array literal
- [x] Inheritance

#### Todo

- lambda/function(block)
- Module (like Ruby's `Module`)
- Enum
- Constants
- Check all ivars will be initialized (like Swift)
- ...

## License

MIT

## Contact

https://github.com/yhara/shiika/issues
